import { useTranslation } from "react-i18next";
import { IconButton } from "../../components/IconButton";
import { Button } from "../../components/Button";
import styles from "./previews.module.css";

export function ZoomControls({
  zoom,
  onZoom,
  min = 0.5,
  max = 3,
}: {
  zoom: number;
  onZoom: (zoom: number) => void;
  min?: number;
  max?: number;
}) {
  const { t } = useTranslation();
  return (
    <div className={styles.zoomControls} role="group" aria-label={t("viewer.zoom")}>
      <IconButton label={t("viewer.zoomOut")} size="sm" disabled={zoom <= min} onClick={() => onZoom(Math.max(min, +(zoom - 0.25).toFixed(2)))}>−</IconButton>
      <Button size="sm" variant="quiet" title={t("viewer.zoomReset")} onClick={() => onZoom(1)}>
        {Math.round(zoom * 100)}%
      </Button>
      <IconButton label={t("viewer.zoomIn")} size="sm" disabled={zoom >= max} onClick={() => onZoom(Math.min(max, +(zoom + 0.25).toFixed(2)))}>+</IconButton>
    </div>
  );
}
