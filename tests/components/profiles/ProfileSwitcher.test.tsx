import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProfileSwitcher } from "@/components/profiles/ProfileSwitcher";

const mutate = vi.fn();

vi.mock("@/lib/query/profiles", () => ({
  useProfilesQuery: () => ({
    data: {
      profiles: [
        {
          id: "largest-profile",
          name: "A profile name that would otherwise consume the header",
          payload: {
            providers: {
              claude: null,
              "claude-desktop": null,
              codex: null,
            },
            mcp: {
              claude: null,
              "claude-desktop": null,
              codex: null,
            },
            skills: {
              claude: null,
              "claude-desktop": null,
              codex: null,
            },
            prompts: {
              claude: null,
              "claude-desktop": null,
              codex: null,
            },
          },
        },
      ],
      currentIds: { claude: "largest-profile" },
    },
  }),
  useApplyProfileMutation: () => ({ mutate }),
  useClearProfileMutation: () => ({ mutate }),
  useCreateProfileMutation: () => ({ isPending: false, mutate }),
}));

vi.mock("@/components/profiles/ProfileManageDialog", () => ({
  ProfileManageDialog: () => null,
}));

describe("ProfileSwitcher", () => {
  it("uses an icon-only but named trigger in constrained headers", () => {
    render(<ProfileSwitcher activeApp="claude" compact />);

    const trigger = screen.getByRole("combobox");
    expect(trigger).toHaveClass("w-8", "justify-center");
    expect(trigger).toHaveAccessibleName("profiles.switcherTooltip.claude");
    expect(
      screen.queryByText(
        "A profile name that would otherwise consume the header",
      ),
    ).not.toBeInTheDocument();
    expect(trigger.querySelector("svg")).not.toBeNull();
  });
});
