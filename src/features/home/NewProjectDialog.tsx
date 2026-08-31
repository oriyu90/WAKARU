import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { Textarea } from "../../components/Textarea";
import { Button } from "../../components/Button";
import { projectsApi } from "../../ipc/projects";
import { IpcError } from "../../ipc/client";

export function NewProjectDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  const create = useMutation({
    mutationFn: () =>
      projectsApi.create({
        name,
        description: description || null,
        color: null,
      }),
    onSuccess: (project) => {
      void qc.invalidateQueries({ queryKey: ["projects"] });
      reset();
      onClose();
      navigate(`/p/${project.id}`);
    },
  });

  function reset() {
    setName("");
    setDescription("");
    create.reset();
  }

  const nameError =
    create.error instanceof IpcError && create.error.code === "PROJECT_NAME_REQUIRED"
      ? t("home.dialog.nameRequired")
      : undefined;

  return (
    <Dialog
      open={open}
      onClose={() => {
        reset();
        onClose();
      }}
      title={t("home.dialog.title")}
      footer={
        <>
          <Button
            variant="quiet"
            onClick={() => {
              reset();
              onClose();
            }}
          >
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            loading={create.isPending}
            disabled={!name.trim()}
            disabledReason={t("home.dialog.nameRequired")}
            onClick={() => create.mutate()}
          >
            {t("home.dialog.create")}
          </Button>
        </>
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate();
        }}
        style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
      >
        <Field label={t("home.dialog.name")} required error={nameError}>
          {({ id, describedBy, invalid }) => (
            <Input
              id={id}
              aria-describedby={describedBy}
              aria-invalid={invalid}
              value={name}
              autoFocus
              onChange={(e) => setName(e.target.value)}
              placeholder={t("home.dialog.namePlaceholder")}
            />
          )}
        </Field>
        <Field label={t("home.dialog.description")} hint={t("home.dialog.optional")}>
          {({ id }) => (
            <Textarea
              id={id}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
            />
          )}
        </Field>
      </form>
    </Dialog>
  );
}
