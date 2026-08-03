import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ProviderActions } from "@/components/providers/ProviderActions";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const onDelete = vi.fn();

function renderCurrentProviderActions(appId: "codex" | "claude") {
  render(
    <ProviderActions
      appId={appId}
      isCurrent
      onSwitch={vi.fn()}
      onEdit={vi.fn()}
      onDuplicate={vi.fn()}
      onDelete={onDelete}
    />,
  );
}

describe("ProviderActions current-provider deletion", () => {
  beforeEach(() => {
    onDelete.mockReset();
  });

  it("allows deleting the current Codex provider through the existing callback", () => {
    renderCurrentProviderActions("codex");

    fireEvent.click(screen.getByTitle("common.delete"));

    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  it("keeps the existing delete prohibition for other current non-additive apps", () => {
    renderCurrentProviderActions("claude");

    fireEvent.click(screen.getByTitle("common.delete"));

    expect(onDelete).not.toHaveBeenCalled();
  });
});
