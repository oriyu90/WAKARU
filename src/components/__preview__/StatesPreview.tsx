/* Dev-only. Visual check of the 8-state contract on every base control
 * (AC-0-4). Not included in production builds — the route is gated on
 * import.meta.env.DEV in router.tsx. */
import { useState } from "react";
import { Button } from "../Button";
import { IconButton } from "../IconButton";
import { Field } from "../Field";
import { Input } from "../Input";
import { Textarea } from "../Textarea";
import { Select } from "../Select";
import { Checkbox } from "../Checkbox";
import { Switch } from "../Switch";
import { Slider } from "../Slider";
import { Tabs, TabPanel } from "../Tabs";
import { Dialog } from "../Dialog";
import { Tooltip } from "../Tooltip";
import { Spinner } from "../Spinner";
import { Skeleton } from "../Skeleton";
import { EmptyState } from "../EmptyState";
import { ErrorState } from "../ErrorState";
import { useToast } from "../useToast";
import { PlusIcon, SettingsIcon } from "../../app/Icons";
import styles from "./StatesPreview.module.css";

const STATES = [
  "default",
  "hover",
  "focus-visible",
  "active",
  "disabled",
  "loading",
  "error",
  "success",
] as const;

export function StatesPreview() {
  const [tab, setTab] = useState("a");
  const [dialog, setDialog] = useState(false);
  const toast = useToast();

  return (
    <div className={styles.page}>
      <h1>Component states</h1>
      <p className={styles.lede}>
        Every interactive primitive, every state (docs/06 §9.1). Tab through with
        the keyboard to see focus rings.
      </p>

      <section>
        <h2>Button — 8 states</h2>
        <div className={styles.row}>
          <Button>{STATES[0]}</Button>
          <Button className={styles.forceHover}>{STATES[1]}</Button>
          <Button autoFocus>{STATES[2]}</Button>
          <Button className={styles.forceActive}>{STATES[3]}</Button>
          <Button disabled disabledReason="Explains why">
            {STATES[4]}
          </Button>
          <Button loading>{STATES[5]}</Button>
          <Button variant="danger">{STATES[6]}</Button>
          <Button success>{STATES[7]}</Button>
        </div>
        <div className={styles.row}>
          <Button variant="primary">primary</Button>
          <Button variant="secondary">secondary</Button>
          <Button variant="quiet">quiet</Button>
          <Button variant="danger">danger</Button>
          <IconButton label="Add">
            <PlusIcon />
          </IconButton>
          <Tooltip content="Opens settings">
            <IconButton label="Settings">
              <SettingsIcon />
            </IconButton>
          </Tooltip>
        </div>
      </section>

      <section>
        <h2>Text input</h2>
        <div className={styles.grid}>
          <Field label="Default" hint="Helper line reserves its space">
            {({ id }) => <Input id={id} placeholder="01 Jan 2026" />}
          </Field>
          <Field label="Error" error="That endpoint is unreachable.">
            {({ id, invalid }) => (
              <Input id={id} aria-invalid={invalid} defaultValue="bad" />
            )}
          </Field>
          <Field label="Disabled">
            {({ id }) => <Input id={id} disabled value="locked" readOnly />}
          </Field>
          <Field label="Textarea">
            {({ id }) => <Textarea id={id} placeholder="Multi-line…" />}
          </Field>
          <Field label="Select">
            {({ id }) => (
              <Select id={id} defaultValue="b">
                <option value="a">Alpha</option>
                <option value="b">Beta</option>
              </Select>
            )}
          </Field>
        </div>
      </section>

      <section>
        <h2>Toggles</h2>
        <div className={styles.row}>
          <Checkbox label="Checkbox" defaultChecked />
          <Checkbox label="Disabled" disabled />
          <Switch label="Switch" defaultChecked />
          <Switch label="Disabled switch" disabled />
        </div>
        <Slider defaultValue={40} aria-label="Example slider" />
      </section>

      <section>
        <h2>Tabs</h2>
        <Tabs
          label="Demo"
          value={tab}
          onChange={setTab}
          items={[
            { id: "a", label: "First" },
            { id: "b", label: "Second" },
            { id: "c", label: "Third" },
          ]}
        />
        <TabPanel id="a" active={tab === "a"}>
          Panel A
        </TabPanel>
        <TabPanel id="b" active={tab === "b"}>
          Panel B
        </TabPanel>
        <TabPanel id="c" active={tab === "c"}>
          Panel C
        </TabPanel>
        <Tabs
          label="Segmented demo"
          variant="segmented"
          value={tab}
          onChange={setTab}
          items={[
            { id: "a", label: "資料を見る" },
            { id: "b", label: "Studio" },
          ]}
        />
      </section>

      <section>
        <h2>Feedback</h2>
        <div className={styles.row}>
          <Spinner label="Loading" />
          <Skeleton width={180} height={12} />
          <Button onClick={() => setDialog(true)}>Open dialog</Button>
          <Button onClick={() => toast.push({ tone: "error", message: "Failed to save." })}>
            Error toast
          </Button>
          <Button
            onClick={() =>
              toast.push({
                tone: "info",
                message: "Deleted.",
                action: { label: "Undo", onClick: () => {} },
              })
            }
          >
            Toast + undo
          </Button>
        </div>
        <EmptyState
          title="Nothing here yet"
          body="This is what an empty list says."
          actions={<Button variant="primary">First action</Button>}
        />
        <ErrorState error={new Error("boom")} onRetry={() => {}} />
      </section>

      <Dialog
        open={dialog}
        onClose={() => setDialog(false)}
        title="Example dialog"
        footer={
          <>
            <Button variant="quiet" onClick={() => setDialog(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={() => setDialog(false)}>
              Confirm
            </Button>
          </>
        }
      >
        Native &lt;dialog&gt; — focus trapped, Esc closes, backdrop dims.
      </Dialog>
    </div>
  );
}
