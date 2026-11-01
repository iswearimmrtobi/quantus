import { useState } from 'react';
import type { SessionInfo } from '../api';
import { useT } from '../i18n';

/**
 * The session menu: the saved sessions, the one used last first. Each keeps its variables,
 * history and mode inputs; the shell writes it to a file of its own after every change.
 */
export function SessionsPanel(props: {
  sessions: SessionInfo[];
  current: string;
  onOpen: (id: string) => void;
  onNew: () => void;
  onRename: (id: string, name: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const { t, language } = useT();
  const [renaming, setRenaming] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [deleting, setDeleting] = useState<string | null>(null);

  const commitName = (id: string) => {
    if (name.trim()) props.onRename(id, name.trim());
    setRenaming(null);
  };

  return (
    <div className="sessions">
      <button className="setting-reset sessions-new" onClick={props.onNew}>
        {t('sessions.new')}
      </button>
      {props.sessions.map((session) => (
        <div className={session.id === props.current ? 'session-item active' : 'session-item'} key={session.id}>
          {renaming === session.id ? (
            <input
              className="library-filter"
              autoFocus
              value={name}
              onChange={(event) => setName(event.target.value)}
              onBlur={() => commitName(session.id)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') commitName(session.id);
                if (event.key === 'Escape') {
                  event.stopPropagation();
                  setRenaming(null);
                }
              }}
            />
          ) : (
            <button className="session-open" onClick={() => props.onOpen(session.id)}>
              <span className="session-name">{session.name}</span>
              <span className="session-meta">
                {t('sessions.entries', { n: session.entries })}
                {session.lastUsed !== null &&
                  ` · ${new Date(session.lastUsed).toLocaleString(language, { dateStyle: 'short', timeStyle: 'short' })}`}
              </span>
            </button>
          )}
          {deleting === session.id ? (
            <div className="session-actions">
              <button className="session-action danger" onClick={() => props.onDelete(session.id)}>
                {t('sessions.confirmDelete')}
              </button>
              <button className="session-action" onClick={() => setDeleting(null)}>
                {t('sessions.cancel')}
              </button>
            </div>
          ) : (
            <div className="session-actions">
              <button
                className="session-action"
                title={t('sessions.rename')}
                onClick={() => {
                  setName(session.name);
                  setRenaming(session.id);
                }}
              >
                ✎
              </button>
              <button className="session-action" title={t('sessions.duplicate')} onClick={() => props.onDuplicate(session.id)}>
                ⧉
              </button>
              <button className="session-action" title={t('sessions.delete')} onClick={() => setDeleting(session.id)}>
                ✕
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
