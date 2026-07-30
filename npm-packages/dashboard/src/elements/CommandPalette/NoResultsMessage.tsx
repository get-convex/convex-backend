import { Link } from "@ui/Link";
import { useFeedbackFormOpen } from "./FeedbackForm";

export function NoResultsMessage({ onClose }: { onClose: () => void }) {
  const [, setFeedbackOpen] = useFeedbackFormOpen();
  return (
    <>
      No results found.
      <span className="text-content-tertiary">
        Didn’t find what you’re looking for?{" "}
        <Link
          href="#"
          onClick={(e) => {
            e.preventDefault();
            // Close the palette first: the feedback form lives outside it, and
            // this releases the Radix focus trap so the form can take focus.
            onClose();
            setFeedbackOpen(true);
          }}
        >
          Send feedback
        </Link>
        .
      </span>
    </>
  );
}
