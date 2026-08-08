use std::collections::{HashMap, HashSet};

pub(crate) const MAX_DIRECTORY_ROWS: usize = 4_096;
pub(crate) const MAX_COMPONENT_ROWS: usize = 32_768;
pub(crate) const MAX_MSI_FIELD_UNITS: u32 = 1_024;

const INSTALL_DIR: &str = "INSTALLDIR";
const REQUIRED_CORE_COMPONENTS: [&str; 4] = [
    "CMP_UninstallShortcut",
    "InstallDirectoryAcl",
    "Path",
    "RegistryEntries",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectoryRow {
    pub(crate) id: String,
    pub(crate) parent: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ComponentRow {
    pub(crate) id: String,
    pub(crate) directory: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClosureError {
    TooManyDirectories,
    TooManyComponents,
    EmptyDirectoryId,
    EmptyDirectoryParent(String),
    EmptyComponentId,
    EmptyComponentDirectory(String),
    DuplicateDirectory(String),
    DuplicateComponent(String),
    MissingDirectory(String),
    DirectoryCycle(String),
    MissingInstallDir,
    MissingCoreComponent(String),
}

pub(crate) fn install_dir_component_ids(
    directories: Vec<DirectoryRow>,
    components: Vec<ComponentRow>,
) -> Result<Vec<String>, ClosureError> {
    if directories.len() > MAX_DIRECTORY_ROWS {
        return Err(ClosureError::TooManyDirectories);
    }
    if components.len() > MAX_COMPONENT_ROWS {
        return Err(ClosureError::TooManyComponents);
    }

    let mut parents = HashMap::with_capacity(directories.len());
    let mut directory_ids = Vec::with_capacity(directories.len());
    for directory in directories {
        if directory.id.trim().is_empty() {
            return Err(ClosureError::EmptyDirectoryId);
        }
        if directory
            .parent
            .as_deref()
            .is_some_and(|parent| parent.trim().is_empty())
        {
            return Err(ClosureError::EmptyDirectoryParent(directory.id));
        }
        if parents
            .insert(directory.id.clone(), directory.parent)
            .is_some()
        {
            return Err(ClosureError::DuplicateDirectory(directory.id));
        }
        directory_ids.push(directory.id);
    }
    if !parents.contains_key(INSTALL_DIR) {
        return Err(ClosureError::MissingInstallDir);
    }

    // Validate the complete rendered Directory graph, including rows that do
    // not currently own a component. Otherwise an unreferenced malformed
    // parent or a cycle through INSTALLDIR could escape the classifier while
    // the verifier claims the whole table was admitted.
    let mut install_directories = HashSet::with_capacity(directory_ids.len());
    for directory in directory_ids {
        if is_install_dir_descendant(&directory, &parents)? {
            install_directories.insert(directory);
        }
    }

    let mut seen_components = HashSet::with_capacity(components.len());
    let mut closure = Vec::new();
    for component in components {
        if component.id.trim().is_empty() {
            return Err(ClosureError::EmptyComponentId);
        }
        if component.directory.trim().is_empty() {
            return Err(ClosureError::EmptyComponentDirectory(component.id));
        }
        if !seen_components.insert(component.id.clone()) {
            return Err(ClosureError::DuplicateComponent(component.id));
        }
        if !parents.contains_key(&component.directory) {
            return Err(ClosureError::MissingDirectory(component.directory));
        }
        if install_directories.contains(&component.directory) {
            closure.push(component.id);
        }
    }

    closure.sort_unstable();
    for required in REQUIRED_CORE_COMPONENTS {
        if closure
            .binary_search_by(|component| component.as_str().cmp(required))
            .is_err()
        {
            return Err(ClosureError::MissingCoreComponent(required.to_owned()));
        }
    }
    Ok(closure)
}

fn is_install_dir_descendant(
    directory: &str,
    parents: &HashMap<String, Option<String>>,
) -> Result<bool, ClosureError> {
    let mut current = directory;
    let mut visited = HashSet::new();
    let mut below_install_dir = false;
    loop {
        if current == INSTALL_DIR {
            below_install_dir = true;
        }
        if !visited.insert(current.to_owned()) {
            return Err(ClosureError::DirectoryCycle(current.to_owned()));
        }
        match parents.get(current) {
            Some(Some(parent)) if !parent.is_empty() => current = parent,
            Some(_) => return Ok(below_install_dir),
            None => return Err(ClosureError::MissingDirectory(current.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(id: &str, parent: Option<&str>) -> DirectoryRow {
        DirectoryRow {
            id: id.to_owned(),
            parent: parent.map(str::to_owned),
        }
    }

    fn component(id: &str, directory: &str) -> ComponentRow {
        ComponentRow {
            id: id.to_owned(),
            directory: directory.to_owned(),
        }
    }

    fn core_components() -> Vec<ComponentRow> {
        REQUIRED_CORE_COMPONENTS
            .iter()
            .map(|id| component(id, INSTALL_DIR))
            .collect()
    }

    #[test]
    fn returns_every_component_below_install_dir_and_excludes_other_roots() {
        let directories = vec![
            directory("TARGETDIR", None),
            directory("ProgramFiles64Folder", Some("TARGETDIR")),
            directory(INSTALL_DIR, Some("ProgramFiles64Folder")),
            directory("Resources", Some(INSTALL_DIR)),
            directory("NestedResources", Some("Resources")),
            directory("CommonAppDataFolder", Some("TARGETDIR")),
        ];
        let mut components = core_components();
        components.extend([
            component("ResourceA", "Resources"),
            component("ResourceB", "NestedResources"),
            component("OutsideInstallDir", "CommonAppDataFolder"),
        ]);

        let closure = install_dir_component_ids(directories, components).unwrap();

        assert_eq!(
            closure,
            vec![
                "CMP_UninstallShortcut",
                "InstallDirectoryAcl",
                "Path",
                "RegistryEntries",
                "ResourceA",
                "ResourceB",
            ]
        );
    }

    #[test]
    fn rejects_duplicate_rows_and_unknown_directory_links() {
        let directories = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
        ];
        let mut duplicate_components = core_components();
        duplicate_components.push(component("Path", INSTALL_DIR));
        assert_eq!(
            install_dir_component_ids(directories.clone(), duplicate_components),
            Err(ClosureError::DuplicateComponent("Path".to_owned()))
        );

        let mut unknown_directory = core_components();
        unknown_directory.push(component("ResourceA", "MissingDirectory"));
        assert_eq!(
            install_dir_component_ids(directories, unknown_directory),
            Err(ClosureError::MissingDirectory(
                "MissingDirectory".to_owned()
            ))
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_directory_identifiers() {
        let mut duplicate_directories = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
        ];
        duplicate_directories.push(directory(INSTALL_DIR, Some("TARGETDIR")));
        assert_eq!(
            install_dir_component_ids(duplicate_directories, core_components()),
            Err(ClosureError::DuplicateDirectory(INSTALL_DIR.to_owned()))
        );

        let directories = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
        ];
        let mut components = core_components();
        components.push(component("", INSTALL_DIR));
        assert_eq!(
            install_dir_component_ids(directories, components),
            Err(ClosureError::EmptyComponentId)
        );
    }

    #[test]
    fn rejects_cycles_and_missing_core_components() {
        let cyclic_directories = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
            directory("CycleA", Some("CycleB")),
            directory("CycleB", Some("CycleA")),
        ];
        let mut cyclic_components = core_components();
        cyclic_components.push(component("Cyclic", "CycleA"));
        assert_eq!(
            install_dir_component_ids(cyclic_directories, cyclic_components),
            Err(ClosureError::DirectoryCycle("CycleA".to_owned()))
        );

        let directories = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
        ];
        let components = core_components()
            .into_iter()
            .filter(|component| component.id != "Path")
            .collect();
        assert_eq!(
            install_dir_component_ids(directories, components),
            Err(ClosureError::MissingCoreComponent("Path".to_owned()))
        );
    }

    #[test]
    fn rejects_malformed_directory_rows_even_when_no_component_references_them() {
        let unknown_parent = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
            directory("Unused", Some("MissingParent")),
        ];
        assert_eq!(
            install_dir_component_ids(unknown_parent, core_components()),
            Err(ClosureError::MissingDirectory("MissingParent".to_owned()))
        );

        let install_dir_cycle = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("InstallChild")),
            directory("InstallChild", Some(INSTALL_DIR)),
        ];
        assert_eq!(
            install_dir_component_ids(install_dir_cycle, core_components()),
            Err(ClosureError::DirectoryCycle(INSTALL_DIR.to_owned()))
        );
    }

    #[test]
    fn rejects_whitespace_only_directory_and_component_fields() {
        let empty_parent = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("   ")),
        ];
        assert_eq!(
            install_dir_component_ids(empty_parent, core_components()),
            Err(ClosureError::EmptyDirectoryParent(INSTALL_DIR.to_owned()))
        );

        let directories = vec![
            directory("TARGETDIR", None),
            directory(INSTALL_DIR, Some("TARGETDIR")),
        ];
        let mut components = core_components();
        components.push(component("   ", INSTALL_DIR));
        assert_eq!(
            install_dir_component_ids(directories, components),
            Err(ClosureError::EmptyComponentId)
        );
    }
}
