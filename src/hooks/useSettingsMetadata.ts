import { useCallback, useEffect, useState } from "react";
import { settingsApi } from "@/lib/api";
import type { RuntimePrivilegeStatus } from "@/lib/api/settings";

export interface UseSettingsMetadataResult {
  isPortable: boolean;
  runtimePrivilege: RuntimePrivilegeStatus | null;
  requiresRestart: boolean;
  isLoading: boolean;
  acknowledgeRestart: () => void;
  setRequiresRestart: (value: boolean) => void;
}

/**
 * useSettingsMetadata - 元数据管理
 * 负责：
 * - isPortable（便携模式）
 * - runtimePrivilege（Windows 运行权限的只读状态）
 * - requiresRestart（需要重启标志）
 */
export function useSettingsMetadata(): UseSettingsMetadataResult {
  const [isPortable, setIsPortable] = useState(false);
  const [runtimePrivilege, setRuntimePrivilege] =
    useState<RuntimePrivilegeStatus | null>(null);
  const [requiresRestart, setRequiresRestart] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  // 加载元数据
  useEffect(() => {
    let active = true;
    setIsLoading(true);

    const load = async () => {
      try {
        const [portableResult, privilegeResult] = await Promise.allSettled([
          settingsApi.isPortable(),
          settingsApi.getRuntimePrivilegeStatus(),
        ]);

        if (!active) return;

        if (portableResult.status === "fulfilled") {
          setIsPortable(portableResult.value);
        } else {
          console.error(
            "[useSettingsMetadata] Failed to load portable metadata",
            portableResult.reason,
          );
        }
        if (privilegeResult.status === "fulfilled") {
          setRuntimePrivilege(privilegeResult.value);
        } else {
          console.error(
            "[useSettingsMetadata] Failed to load runtime privilege metadata",
            privilegeResult.reason,
          );
        }
      } catch (error) {
        console.error("[useSettingsMetadata] Failed to load metadata", error);
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, []);

  const acknowledgeRestart = useCallback(() => {
    setRequiresRestart(false);
  }, []);

  return {
    isPortable,
    runtimePrivilege,
    requiresRestart,
    isLoading,
    acknowledgeRestart,
    setRequiresRestart,
  };
}
