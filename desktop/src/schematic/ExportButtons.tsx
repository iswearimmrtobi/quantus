import { useState } from 'react';
import { useT } from '../i18n';
import { saveBytes } from '../shell';
import { pngBytes, svgBytes, type Picture } from './picture';

/** SVG and PNG under the palette: the drawing saved as a picture file (§10.5). */
export function ExportButtons({ picture, name, disabled }: { picture(): Picture; name: string; disabled: boolean }) {
  const { t } = useT();
  const [error, setError] = useState<string | null>(null);
  const save = async (format: 'svg' | 'png') => {
    try {
      const drawn = picture();
      const bytes = format === 'svg' ? svgBytes(drawn) : await pngBytes(drawn);
      await saveBytes(bytes, `${name}.${format}`, { name: format.toUpperCase(), extensions: [format] });
      setError(null);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    }
  };
  return (
    <div className="schematic-export">
      <div className="palette-title">{t('schematic.export.title')}</div>
      <div className="schematic-export-buttons">
        <button className="form-chip" title={t('schematic.export.svg.hint')} disabled={disabled} onClick={() => void save('svg')}>
          SVG
        </button>
        <button className="form-chip" title={t('schematic.export.png.hint')} disabled={disabled} onClick={() => void save('png')}>
          PNG
        </button>
      </div>
      {error && <p className="file-error">{error}</p>}
    </div>
  );
}
