import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DatabaseUpgrade } from "@/components/DatabaseUpgrade";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  exit: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  exit: mocks.exit,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: string | Record<string, unknown>): string => {
      if (typeof options === "string") {
        return options;
      }
      const template =
        typeof options?.defaultValue === "string" ? options.defaultValue : key;
      return template.replace(/{{(\w+)}}/g, (_, variable: string) =>
        String(options?.[variable] ?? `{{${variable}}}`),
      );
    },
  }),
}));

describe("DatabaseUpgrade", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.exit.mockReset();
  });

  it("keeps a newer database blocked without checking or installing updates", () => {
    render(
      <DatabaseUpgrade
        payload={{
          kind: "db_version_too_new",
          db_version: 9,
          supported_version: 7,
          path: "/safe/config.db",
        }}
      />,
    );

    expect(screen.getByText("需要兼容的 FyAgent 构建版本")).toBeInTheDocument();
    expect(screen.getByText(/数据库将保持不变/)).toBeInTheDocument();
    expect(screen.getByText(/数据库版本 v9 · 应用支持 v7/)).toBeInTheDocument();
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: /升级|更新|发布页/ }),
    ).not.toBeInTheDocument();
    expect(document.querySelector("a[href]")).toBeNull();
  });

  it("only exposes local configuration access and quit actions", async () => {
    const user = userEvent.setup();
    render(<DatabaseUpgrade payload={{ kind: "db_version_too_new" }} />);

    await user.click(screen.getByRole("button", { name: "打开配置目录" }));
    expect(mocks.invoke).toHaveBeenCalledWith("open_app_config_folder");

    await user.click(screen.getByRole("button", { name: "退出" }));
    expect(mocks.exit).toHaveBeenCalledWith(0);
  });
});
