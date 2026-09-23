import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { useNavigation } from "@/stores/navigation";
import { useArchiveInterest, useInterestSuggestion } from "./useInterests";
import styles from "./InterestSuggestion.module.css";

/** "Something you wanted to explore": one forgotten interest a day, on My Day. */
export function InterestSuggestion() {
  const { data } = useInterestSuggestion();
  const archive = useArchiveInterest();
  const go = useNavigation((s) => s.go);

  if (!data) return null;
  return (
    <Card title="Something you wanted to explore" labelledBy="card-interest">
      <p>{data.text}</p>
      <div className={styles.actions}>
        <Button
          variant="ghost"
          aria-label={`Archive ${data.text}`}
          onClick={() => archive.mutate(data.id)}
        >
          Archive
        </Button>
        <Button variant="secondary" onClick={() => go("interest-inbox")}>
          Open Interest Inbox
        </Button>
      </div>
    </Card>
  );
}
