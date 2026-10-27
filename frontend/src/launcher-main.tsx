import { createRoot } from "react-dom/client";
import { LauncherHome } from "./LauncherHome";
import { bootstrapHost } from "./launcher-api";
import "./styles.css";

const root = document.getElementById("root");
if (root) {
  const application = createRoot(root);
  let generation = 0;
  const open = () => {
    let initialLink: string | null = null,
      initialError: string | null = null;
    try {
      initialLink = bootstrapHost();
    } catch (error) {
      initialError =
        error instanceof Error
          ? error.message
          : "The launcher link is unavailable. Run cube play again.";
    }
    application.render(
      <LauncherHome
        key={generation++}
        initialLink={initialLink}
        initialError={initialError}
      />,
    );
  };
  window.addEventListener("hashchange", open);
  open();
}
