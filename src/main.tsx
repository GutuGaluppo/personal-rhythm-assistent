import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Root } from "@/app/root";
import { Providers } from "@/app/providers";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@/design-system/tokens/tokens.css";
import "@/design-system/tokens/base.css";
import { applyMotionPreference } from "@/lib/utils/motion";

applyMotionPreference();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Providers>
      <Root />
    </Providers>
  </StrictMode>,
);
