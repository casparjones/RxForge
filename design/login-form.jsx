// Functional login form for RxForge splash + popup screens.
// Real state, validation, OAuth provider buttons, error/success transitions.

const { useState } = React;

const EMBER = "#7c7cff";       // indigo accent (was ember-orange)
const BG = "#0e0f1a";           // near-black indigo
const SURFACE = "#161829";
const BORDER = "#22253a";
const BORDER_HI = "#2e3247";
const TEXT = "#eef0fa";
const MUTED = "#8b8fa8";

const inputStyle = (focus, error) => ({
  width: '100%',
  background: '#0c0d18',
  border: `1px solid ${error ? '#a13a4a' : focus ? EMBER : BORDER_HI}`,
  borderRadius: 6,
  padding: '12px 14px',
  color: TEXT,
  fontSize: 14,
  fontFamily: '"JetBrains Mono", ui-monospace, monospace',
  outline: 'none',
  transition: 'border-color 120ms ease, box-shadow 120ms ease',
  boxShadow: focus ? `0 0 0 3px ${EMBER}22` : 'none',
  boxSizing: 'border-box',
});

const labelStyle = {
  fontFamily: '"JetBrains Mono", monospace',
  fontSize: 11,
  letterSpacing: '0.08em',
  textTransform: 'uppercase',
  color: MUTED,
  marginBottom: 6,
  display: 'block',
};

const ProviderIcon = ({ kind }) => {
  if (kind === 'github') return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2c-3.2.7-3.87-1.36-3.87-1.36-.52-1.33-1.28-1.69-1.28-1.69-1.05-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.76 2.7 1.25 3.36.95.1-.74.4-1.25.73-1.54-2.55-.29-5.24-1.27-5.24-5.66 0-1.25.45-2.27 1.18-3.07-.12-.29-.51-1.46.11-3.04 0 0 .96-.31 3.16 1.17.92-.26 1.9-.39 2.88-.39s1.96.13 2.88.39c2.2-1.48 3.16-1.17 3.16-1.17.62 1.58.23 2.75.11 3.04.74.8 1.18 1.82 1.18 3.07 0 4.4-2.69 5.37-5.25 5.65.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56C20.21 21.39 23.5 17.08 23.5 12 23.5 5.65 18.35.5 12 .5z"/></svg>
  );
  if (kind === 'google') return (
    <svg width="16" height="16" viewBox="0 0 24 24"><path fill="#4285F4" d="M22.5 12.27c0-.79-.07-1.54-.2-2.27H12v4.51h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.22-4.74 3.22-8.32z"/><path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.99.66-2.25 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/><path fill="#FBBC05" d="M5.84 14.1c-.22-.66-.35-1.37-.35-2.1s.13-1.44.35-2.1V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l3.66-2.84z"/><path fill="#EA4335" d="M12 5.38c1.62 0 3.07.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84C6.71 7.31 9.14 5.38 12 5.38z"/></svg>
  );
  if (kind === 'gitlab') return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="#FC6D26"><path d="M23.6 9.6L20.3 0 16.6 9.6h-9L4 0 .4 9.6c-.3.7 0 1.5.5 1.9L12 20l11-8.5c.6-.5.9-1.2.6-1.9z"/></svg>
  );
  return null;
};

function LoginForm({ compact = false, onSubmit }) {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [focus, setFocus] = useState(null);
  const [error, setError] = useState(null);
  const [submitting, setSubmitting] = useState(false);
  const [success, setSuccess] = useState(false);

  const handleSubmit = (e) => {
    e.preventDefault();
    setError(null);
    if (!email.includes('@')) { setError('Enter a valid email'); return; }
    if (password.length < 6) { setError('Password must be at least 6 characters'); return; }
    setSubmitting(true);
    setTimeout(() => {
      setSubmitting(false);
      setSuccess(true);
      onSubmit?.({ email });
    }, 900);
  };

  const gap = compact ? 12 : 16;

  return (
    <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap, width: '100%' }}>
      {/* OAuth providers */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {['github', 'google', 'gitlab'].map(p => (
          <button
            key={p}
            type="button"
            style={{
              display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10,
              background: '#0c0d18', color: TEXT,
              border: `1px solid ${BORDER_HI}`, borderRadius: 6,
              padding: compact ? '10px 14px' : '12px 16px',
              fontFamily: '"Space Grotesk", sans-serif', fontSize: 14, fontWeight: 500,
              cursor: 'pointer', transition: 'border-color 120ms, background 120ms',
            }}
            onMouseEnter={e => { e.currentTarget.style.borderColor = EMBER; }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = BORDER_HI; }}
          >
            <ProviderIcon kind={p} />
            <span>Continue with {p === 'github' ? 'GitHub' : p === 'google' ? 'Google' : 'GitLab'}</span>
          </button>
        ))}
      </div>

      {/* divider */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, color: MUTED, fontFamily: '"JetBrains Mono", monospace', fontSize: 10, letterSpacing: '0.15em' }}>
        <div style={{ flex: 1, height: 1, background: BORDER }}/>
        <span>OR</span>
        <div style={{ flex: 1, height: 1, background: BORDER }}/>
      </div>

      {/* email */}
      <div>
        <label style={labelStyle}>Email</label>
        <input
          type="email"
          value={email}
          onChange={e => setEmail(e.target.value)}
          onFocus={() => setFocus('email')}
          onBlur={() => setFocus(null)}
          placeholder="dev@example.com"
          style={inputStyle(focus === 'email', error && error.includes('email'))}
        />
      </div>

      {/* password */}
      <div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
          <label style={labelStyle}>Password</label>
          <a href="#" style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: EMBER, textDecoration: 'none' }}>Forgot?</a>
        </div>
        <input
          type="password"
          value={password}
          onChange={e => setPassword(e.target.value)}
          onFocus={() => setFocus('password')}
          onBlur={() => setFocus(null)}
          placeholder="••••••••••"
          style={inputStyle(focus === 'password', error && error.includes('Password'))}
        />
      </div>

      {error && (
        <div style={{
          fontFamily: '"JetBrains Mono", monospace', fontSize: 12,
          color: '#ff9ab0', background: '#2a1422', border: '1px solid #4a2034',
          borderRadius: 4, padding: '8px 12px',
        }}>
          ! {error}
        </div>
      )}
      {success && (
        <div style={{
          fontFamily: '"JetBrains Mono", monospace', fontSize: 12,
          color: '#9aff9a', background: '#0f2a1a', border: '1px solid #1f4a2a',
          borderRadius: 4, padding: '8px 12px',
        }}>
          ✓ Provisioning CouchDB instance for {email}…
        </div>
      )}

      <button
        type="submit"
        disabled={submitting || success}
        style={{
          background: success ? '#1f4a2a' : EMBER,
          color: success ? '#9aff9a' : '#0a0a18',
          border: 'none', borderRadius: 6,
          padding: compact ? '12px' : '14px',
          fontFamily: '"Space Grotesk", sans-serif', fontWeight: 600, fontSize: 14,
          letterSpacing: '0.01em', cursor: submitting ? 'wait' : 'pointer',
          transition: 'background 120ms, transform 80ms',
          opacity: submitting ? 0.7 : 1,
        }}
        onMouseDown={e => !submitting && (e.currentTarget.style.transform = 'scale(0.98)')}
        onMouseUp={e => (e.currentTarget.style.transform = 'scale(1)')}
      >
        {submitting ? 'Authenticating…' : success ? 'Authenticated' : 'Sign in to RxForge'}
      </button>

      <div style={{ textAlign: 'center', fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: MUTED }}>
        New here? <a href="#" style={{ color: TEXT, textDecoration: 'none', borderBottom: `1px dotted ${MUTED}` }}>Create an account</a>
      </div>
    </form>
  );
}

window.LoginForm = LoginForm;
window.RX_COLORS = { EMBER, BG, SURFACE, BORDER, BORDER_HI, TEXT, MUTED };
