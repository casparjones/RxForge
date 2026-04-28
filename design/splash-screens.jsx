// RxForge splash screen artboards — widescreen, portrait, popup, plus logo specimens
const { EMBER, BG, SURFACE, BORDER, BORDER_HI, TEXT, MUTED } = window.RX_COLORS;

// Decorative status row — small build/version chip used in splash chrome
const StatusRow = ({ size = 1 }) => (
  <div style={{
    display: 'inline-flex', alignItems: 'center', gap: 8 * size,
    fontFamily: '"JetBrains Mono", monospace', fontSize: 11 * size,
    color: MUTED, letterSpacing: '0.05em',
  }}>
    <span style={{ width: 6 * size, height: 6 * size, borderRadius: '50%', background: '#7ad97a', boxShadow: `0 0 ${8*size}px #7ad97a` }}/>
    <span>backend.rxforge.dev</span>
    <span style={{ opacity: 0.4 }}>·</span>
    <span>v1.4.2</span>
    <span style={{ opacity: 0.4 }}>·</span>
    <span>region: eu-fra</span>
  </div>
);

// Sidecar copy block — feature bullets
const SideCopy = ({ scale = 1 }) => {
  const items = [
    { k: 'OAUTH 2.0', v: 'GitHub, Google, GitLab providers wired in.' },
    { k: 'PER-USER COUCHDB', v: 'Provisioned on first login. Isolated.' },
    { k: 'RXDB PLUGIN', v: 'Drop-in replication for your client.' },
    { k: 'SELF-HOSTED', v: 'Your servers, your keys, your data.' },
  ];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 22 * scale }}>
      {items.map((it, i) => (
        <div key={i} style={{ display: 'flex', gap: 16 * scale }}>
          <div style={{
            fontFamily: '"JetBrains Mono", monospace', fontSize: 11 * scale,
            color: EMBER, letterSpacing: '0.1em', minWidth: 130 * scale,
            paddingTop: 4 * scale,
          }}>
            {String(i+1).padStart(2,'0')} · {it.k}
          </div>
          <div style={{
            fontFamily: '"Space Grotesk", sans-serif', fontSize: 16 * scale,
            color: TEXT, lineHeight: 1.45, maxWidth: 340 * scale,
          }}>
            {it.v}
          </div>
        </div>
      ))}
    </div>
  );
};

// ─────────────────────────────────────────────────────────────────
// LOGO SPECIMEN — shows all 4 marks in light + dark contexts
// ─────────────────────────────────────────────────────────────────
function LogoSpecimen() {
  const concepts = [
    { name: 'A · Anvil Spark', Mark: window.MarkAnvilSpark, primary: true },
    { name: 'B · Hex Forge',   Mark: window.MarkHexForge },
    { name: 'C · Loop Forge',  Mark: window.MarkLoopForge },
    { name: 'D · Ingot',       Mark: window.MarkIngot },
  ];
  return (
    <div style={{ width: 1400, padding: 60, background: BG, color: TEXT, fontFamily: '"Space Grotesk", sans-serif' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 40 }}>
        <div>
          <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: EMBER, letterSpacing: '0.15em', marginBottom: 8 }}>01 · LOGO SYSTEM</div>
          <div style={{ fontSize: 32, fontWeight: 600, letterSpacing: '-0.02em' }}>Mark explorations</div>
        </div>
        <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 12, color: MUTED }}>4 concepts · ember on graphite</div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 20 }}>
        {concepts.map(({ name, Mark, primary }) => (
          <div key={name} style={{
            background: SURFACE, border: `1px solid ${primary ? EMBER : BORDER}`,
            borderRadius: 8, padding: 32, position: 'relative',
          }}>
            {primary && (
              <div style={{
                position: 'absolute', top: 12, right: 12,
                fontFamily: '"JetBrains Mono", monospace', fontSize: 10,
                color: EMBER, letterSpacing: '0.1em',
              }}>★ PRIMARY</div>
            )}
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: 120, marginBottom: 20 }}>
              <Mark size={96} ember={EMBER} base={TEXT} />
            </div>
            <div style={{ borderTop: `1px solid ${BORDER}`, paddingTop: 16 }}>
              <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: MUTED, marginBottom: 8 }}>{name}</div>
              <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                <Mark size={28} ember={EMBER} base={TEXT} />
                <Mark size={20} ember={EMBER} base={TEXT} />
                <Mark size={14} ember={EMBER} base={TEXT} />
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Lockups */}
      <div style={{ marginTop: 60, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20 }}>
        <div style={{ background: BG, border: `1px solid ${BORDER}`, borderRadius: 8, padding: 60, display: 'flex', flexDirection: 'column', gap: 32, alignItems: 'flex-start' }}>
          <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: MUTED, letterSpacing: '0.1em' }}>DARK / PRIMARY</div>
          <window.Lockup color={TEXT} ember={EMBER} size={56} />
          <window.Lockup color={TEXT} ember={EMBER} size={36} />
          <window.Lockup color={TEXT} ember={EMBER} size={22} />
        </div>
        <div style={{ background: '#f5efe6', border: `1px solid ${BORDER}`, borderRadius: 8, padding: 60, display: 'flex', flexDirection: 'column', gap: 32, alignItems: 'flex-start' }}>
          <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: '#6b5d4a', letterSpacing: '0.1em' }}>LIGHT / INVERSE</div>
          <window.Lockup color="#1a1410" ember={EMBER} size={56} />
          <window.Lockup color="#1a1410" ember={EMBER} size={36} />
          <window.Lockup color="#1a1410" ember={EMBER} size={22} />
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────
// WIDESCREEN SPLASH (1920×1080) — login on right, brand on left
// ─────────────────────────────────────────────────────────────────
function SplashWide() {
  return (
    <div style={{
      width: 1920, height: 1080, background: BG, color: TEXT,
      fontFamily: '"Space Grotesk", sans-serif', display: 'flex',
      position: 'relative', overflow: 'hidden',
    }}>
      {/* corner ticks */}
      <CornerTicks />

      {/* LEFT — brand */}
      <div style={{ flex: '1.3 1 0', padding: '80px 100px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <window.Lockup color={TEXT} ember={EMBER} size={36} />
          <StatusRow />
        </div>

        <div>
          <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 13, color: EMBER, letterSpacing: '0.18em', marginBottom: 28 }}>
            ━━ AUTH · v1.4.2
          </div>
          <h1 style={{
            fontSize: 96, fontWeight: 500, lineHeight: 0.98,
            letterSpacing: '-0.035em', margin: 0, maxWidth: 820,
            textWrap: 'balance',
          }}>
            Self-hosted sync<br/>
            <span style={{ color: MUTED }}>for </span>
            <span style={{ color: EMBER }}>RxDB</span>
            <span style={{ color: MUTED }}> apps.</span>
          </h1>
          <p style={{
            fontSize: 20, color: MUTED, lineHeight: 1.5,
            maxWidth: 560, marginTop: 32, marginBottom: 60,
          }}>
            OAuth 2.0, per-user CouchDB provisioning, and a TypeScript plugin for seamless replication. Forge your own backend.
          </p>
          <SideCopy />
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end', fontFamily: '"JetBrains Mono", monospace', fontSize: 12, color: MUTED }}>
          <span>© 2026 RxForge · MIT licensed</span>
          <span>docs.rxforge.dev</span>
        </div>
      </div>

      {/* RIGHT — login card */}
      <div style={{
        flex: '1 1 0', background: '#100d09',
        borderLeft: `1px solid ${BORDER}`,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        padding: 80,
      }}>
        <div style={{
          width: '100%', maxWidth: 440,
          background: SURFACE, border: `1px solid ${BORDER_HI}`,
          borderRadius: 12, padding: 48,
        }}>
          <div style={{ marginBottom: 32 }}>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: EMBER, letterSpacing: '0.15em', marginBottom: 12 }}>
              ─ SIGN IN
            </div>
            <div style={{ fontSize: 28, fontWeight: 600, letterSpacing: '-0.02em' }}>Welcome back</div>
            <div style={{ color: MUTED, fontSize: 14, marginTop: 6 }}>
              Authenticate to sync your local-first data.
            </div>
          </div>
          <window.LoginForm />
        </div>
      </div>
    </div>
  );
}

// Corner ticks decoration shared across splashes
function CornerTicks() {
  const tick = { position: 'absolute', width: 28, height: 28, borderColor: BORDER_HI, borderStyle: 'solid' };
  return (
    <>
      <div style={{ ...tick, top: 32, left: 32, borderWidth: '1px 0 0 1px' }}/>
      <div style={{ ...tick, top: 32, right: 32, borderWidth: '1px 1px 0 0' }}/>
      <div style={{ ...tick, bottom: 32, left: 32, borderWidth: '0 0 1px 1px' }}/>
      <div style={{ ...tick, bottom: 32, right: 32, borderWidth: '0 1px 1px 0' }}/>
    </>
  );
}

// ─────────────────────────────────────────────────────────────────
// PORTRAIT SPLASH (1080×1920) — mobile / tall layout
// ─────────────────────────────────────────────────────────────────
function SplashPortrait() {
  return (
    <div style={{
      width: 1080, height: 1920, background: BG, color: TEXT,
      fontFamily: '"Space Grotesk", sans-serif',
      display: 'flex', flexDirection: 'column', position: 'relative', overflow: 'hidden',
    }}>
      <CornerTicks />

      {/* top brand */}
      <div style={{ padding: '80px 80px 0', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <window.Lockup color={TEXT} ember={EMBER} size={44} />
        <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 14, color: MUTED, letterSpacing: '0.1em' }}>v1.4.2</div>
      </div>

      {/* hero */}
      <div style={{ padding: '120px 80px 60px' }}>
        <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 16, color: EMBER, letterSpacing: '0.18em', marginBottom: 32 }}>
          ━━ AUTH
        </div>
        <h1 style={{
          fontSize: 110, fontWeight: 500, lineHeight: 0.95,
          letterSpacing: '-0.035em', margin: 0,
          textWrap: 'balance',
        }}>
          Forge your<br/>
          <span style={{ color: EMBER }}>sync</span> backend.
        </h1>
        <p style={{ fontSize: 24, color: MUTED, lineHeight: 1.5, marginTop: 36, maxWidth: 720 }}>
          Self-hosted sync for RxDB apps. OAuth 2.0 + per-user CouchDB.
        </p>
      </div>

      {/* login card */}
      <div style={{ padding: '40px 80px', flex: 1, display: 'flex', alignItems: 'flex-start' }}>
        <div style={{
          width: '100%',
          background: SURFACE, border: `1px solid ${BORDER_HI}`,
          borderRadius: 16, padding: 56,
        }}>
          <div style={{ marginBottom: 36 }}>
            <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 14, color: EMBER, letterSpacing: '0.15em', marginBottom: 12 }}>
              ─ SIGN IN
            </div>
            <div style={{ fontSize: 36, fontWeight: 600, letterSpacing: '-0.02em' }}>Welcome back</div>
          </div>
          <div style={{ transform: 'scale(1.4)', transformOrigin: 'top left', width: 'calc(100% / 1.4)' }}>
            <window.LoginForm />
          </div>
        </div>
      </div>

      {/* footer */}
      <div style={{ padding: '40px 80px 80px', display: 'flex', justifyContent: 'space-between', fontFamily: '"JetBrains Mono", monospace', fontSize: 14, color: MUTED }}>
        <span>backend.rxforge.dev</span>
        <span>© 2026 · MIT</span>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────
// POPUP / MODAL (480×640) — compact OAuth-style window
// ─────────────────────────────────────────────────────────────────
function SplashPopup() {
  return (
    <div style={{
      width: 480, height: 640, background: SURFACE,
      border: `1px solid ${BORDER_HI}`, borderRadius: 12,
      color: TEXT, fontFamily: '"Space Grotesk", sans-serif',
      display: 'flex', flexDirection: 'column', overflow: 'hidden',
      boxShadow: '0 30px 80px rgba(0,0,0,0.6)',
    }}>
      {/* title bar */}
      <div style={{
        padding: '14px 18px',
        borderBottom: `1px solid ${BORDER}`,
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        background: '#100d09',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontFamily: '"JetBrains Mono", monospace', fontSize: 11, color: MUTED }}>
          <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#7ad97a' }}/>
          backend.rxforge.dev
        </div>
        <div style={{ fontFamily: '"JetBrains Mono", monospace', fontSize: 14, color: MUTED, cursor: 'pointer' }}>×</div>
      </div>

      {/* body */}
      <div style={{ padding: '36px 36px 24px', flex: 1, display: 'flex', flexDirection: 'column' }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14, marginBottom: 28 }}>
          <window.MarkAnvilSpark size={48} ember={EMBER} base={TEXT} />
          <div style={{ fontSize: 20, fontWeight: 600, letterSpacing: '-0.01em' }}>Sign in to RxForge</div>
          <div style={{ fontSize: 13, color: MUTED, textAlign: 'center', maxWidth: 320, lineHeight: 1.5 }}>
            Authorize this device to sync via your self-hosted backend.
          </div>
        </div>
        <window.LoginForm compact />
      </div>

      {/* footer */}
      <div style={{
        padding: '12px 18px', borderTop: `1px solid ${BORDER}`,
        background: '#100d09',
        display: 'flex', justifyContent: 'space-between',
        fontFamily: '"JetBrains Mono", monospace', fontSize: 10, color: MUTED,
        letterSpacing: '0.05em',
      }}>
        <span>OAuth 2.0 · TLS</span>
        <span>v1.4.2</span>
      </div>
    </div>
  );
}

Object.assign(window, { LogoSpecimen, SplashWide, SplashPortrait, SplashPopup });
