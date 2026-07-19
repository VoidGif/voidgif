import { useState } from "react";
import { promptSaveProject } from "../lib/saveFlow";
import { useT } from "../stores/settingsStore";
import ToolbarButton from "./ToolbarButton";
import { IconCheck, IconSave, IconX } from "./icons";

export default function SaveProjectButton() {
  const t = useT();
  const [state, setState] = useState<"idle" | "saving" | "saved" | "error">("idle");

  const doSave = async () => {
    const result = await promptSaveProject(() => setState("saving"));
    if (result === "cancelled") return;
    setState(result);
    setTimeout(() => setState("idle"), result === "saved" ? 2000 : 3000);
  };

  const label =
    state === "saving"
      ? t("saving")
      : state === "saved"
        ? t("saved")
        : state === "error"
          ? t("saveFailed")
          : t("save");

  return (
    <ToolbarButton
      label={label}
      desc={t("tipSaveDesc")}
      align="end"
      disabled={state === "saving"}
      onClick={() => void doSave()}
    >
      {state === "saved" ? (
        <IconCheck size={17} className="text-emerald-400" />
      ) : state === "error" ? (
        <IconX size={17} className="text-rose-400" />
      ) : (
        <IconSave size={17} className={state === "saving" ? "animate-pulse" : undefined} />
      )}
    </ToolbarButton>
  );
}
