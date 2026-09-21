import { ContextDebug } from "@/features/debug/ContextDebug";
import { InterestInbox } from "@/features/interest-inbox/InterestInbox";
import { MyDay } from "@/features/my-day/MyDay";
import { Settings } from "@/features/settings/Settings";
import { useNavigation } from "@/stores/navigation";

export function Page() {
  const page = useNavigation((s) => s.page);
  switch (page) {
    case "my-day":
      return <MyDay />;
    case "interest-inbox":
      return <InterestInbox />;
    case "settings":
      return <Settings />;
    case "context-debug":
      return <ContextDebug />;
  }
}
