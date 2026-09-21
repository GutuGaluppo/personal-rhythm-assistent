import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Root } from "@/app/root";
import { Providers } from "@/app/providers";
import "@/design-system/tokens/tokens.css";
import "@/design-system/tokens/base.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Providers>
      <Root />
    </Providers>
  </StrictMode>,
);
