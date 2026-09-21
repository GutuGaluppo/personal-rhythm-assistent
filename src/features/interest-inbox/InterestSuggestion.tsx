import { Card } from "@/components/ui/Card";
import { useNavigation } from "@/stores/navigation";
import { useArchiveInterest, useInterestSuggestion } from "./useInterests";

/** "Something you wanted to explore": one forgotten interest a day, on My Day. */
export function InterestSuggestion() {
  const { data } = useInterestSuggestion();
  const archive = useArchiveInterest();
  const go = useNavigation((s) => s.go);

  if (!data) return null;
  return (
    <Card title="Something you wanted to explore" labelledBy="card-interest">
      <p>{data.text}</p>
      <div style={{ display: "flex", gap: "var(--space-2)" }}>
        <button
          type="button"
          aria-label={`Archive ${data.text}`}
          onClick={() => archive.mutate(data.id)}
        >
          Archive
        </button>
        <button type="button" onClick={() => go("interest-inbox")}>
          Open Interest Inbox
        </button>
      </div>
    </Card>
  );
}
