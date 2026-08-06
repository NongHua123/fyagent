import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
interface WorkBuddyDuplicateConflictDialogProps {
  existingIds: string[];
  isOpen: boolean;
  isSaving: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export function WorkBuddyDuplicateConflictDialog({
  existingIds,
  isOpen,
  isSaving,
  onCancel,
  onConfirm,
}: WorkBuddyDuplicateConflictDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => {
        if (!open && !isSaving) onCancel();
      }}
    >
      <DialogContent className="max-w-md" zIndex="alert">
        <DialogHeader className="space-y-2 border-b-0 bg-transparent pb-0">
          <DialogTitle className="flex items-center gap-2 text-base font-semibold">
            <AlertTriangle className="h-5 w-5 text-yellow-500" />
            {t("workbuddy.duplicateDialog.title")}
          </DialogTitle>
          <DialogDescription className="text-sm leading-relaxed">
            {t("workbuddy.duplicateDialog.description")}
          </DialogDescription>
        </DialogHeader>

        <ul className="max-h-48 space-y-1 overflow-y-auto rounded-md border border-yellow-500/20 bg-yellow-500/5 p-3 text-sm">
          {existingIds.map((id) => (
            <li key={id} className="min-w-0">
              <code className="block truncate font-mono text-xs">{id}</code>
            </li>
          ))}
        </ul>

        <p className="text-xs leading-relaxed text-muted-foreground">
          {t("workbuddy.duplicateDialog.keepDuplicates")}
        </p>

        <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
          <Button
            variant="outline"
            disabled={isSaving}
            onClick={onCancel}
            autoFocus
          >
            {t("common.cancel")}
          </Button>
          <Button disabled={isSaving} onClick={onConfirm}>
            {isSaving
              ? t("workbuddy.saving")
              : t("workbuddy.duplicateDialog.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
