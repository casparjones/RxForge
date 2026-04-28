// RxForge logo marks — 4 abstract concepts referencing forge + sync
// All marks are designed on a 64x64 grid for crisp scaling.

// CONCEPT A: "Anvil Spark" — anvil silhouette with sync arc + spark dot
const MarkAnvilSpark = ({ size = 64, ember = "#ff6a1f", base = "#f5efe6", strokeW = 3 }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
    {/* anvil base */}
    <path d="M14 44 L50 44 L46 52 L18 52 Z" fill={base}/>
    {/* anvil top with horn */}
    <path d="M10 28 L54 28 L50 36 L14 36 Z" fill={base}/>
    {/* sync arc - top */}
    <path d="M20 22 A12 12 0 0 1 44 22" stroke={ember} strokeWidth={strokeW} strokeLinecap="round" fill="none"/>
    {/* arrow head */}
    <path d="M44 22 L40 18 M44 22 L40 26" stroke={ember} strokeWidth={strokeW} strokeLinecap="round"/>
    {/* spark */}
    <circle cx="32" cy="14" r="2.5" fill={ember}/>
  </svg>
);

// CONCEPT B: "Hex Forge" — hexagon with R-glyph + ember inside
const MarkHexForge = ({ size = 64, ember = "#ff6a1f", base = "#f5efe6", strokeW = 3 }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
    {/* outer hex */}
    <path d="M32 4 L56 18 L56 46 L32 60 L8 46 L8 18 Z" stroke={base} strokeWidth={strokeW} strokeLinejoin="round" fill="none"/>
    {/* inner ember triangle (anvil tip / flame) */}
    <path d="M32 22 L42 42 L22 42 Z" fill={ember}/>
    {/* sync notch top */}
    <circle cx="32" cy="16" r="2.5" fill={base}/>
  </svg>
);

// CONCEPT C: "Loop Forge" — two interlocking arrows forming an X / forge cross
const MarkLoopForge = ({ size = 64, ember = "#ff6a1f", base = "#f5efe6", strokeW = 4 }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
    {/* outer ring */}
    <circle cx="32" cy="32" r="26" stroke={base} strokeWidth={strokeW * 0.6} fill="none" opacity="0.35"/>
    {/* sync arrow top */}
    <path d="M14 24 Q32 14 50 24" stroke={base} strokeWidth={strokeW} strokeLinecap="round" fill="none"/>
    <path d="M50 24 L44 20 M50 24 L46 30" stroke={base} strokeWidth={strokeW} strokeLinecap="round"/>
    {/* sync arrow bottom — ember */}
    <path d="M50 40 Q32 50 14 40" stroke={ember} strokeWidth={strokeW} strokeLinecap="round" fill="none"/>
    <path d="M14 40 L20 44 M14 40 L18 34" stroke={ember} strokeWidth={strokeW} strokeLinecap="round"/>
  </svg>
);

// CONCEPT E: "Bolt" — lightning bolt outline (matches reference UI)
const MarkBolt = ({ size = 64, ember = "#7c7cff", base = "#7c7cff", strokeW = 3 }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
    <path
      d="M36 6 L14 36 L28 36 L24 58 L48 26 L34 26 L36 6 Z"
      stroke={ember}
      strokeWidth={strokeW}
      strokeLinejoin="round"
      strokeLinecap="round"
      fill="none"
    />
  </svg>
);

// CONCEPT D: "Ingot" — stacked plates / database ingot with ember underline
const MarkIngot = ({ size = 64, ember = "#ff6a1f", base = "#f5efe6", strokeW = 3 }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
    {/* three stacked plates — DB cylinders flattened to trapezoids */}
    <path d="M14 16 L50 16 L46 22 L18 22 Z" fill={base}/>
    <path d="M16 26 L48 26 L44 32 L20 32 Z" fill={base} opacity="0.75"/>
    <path d="M18 36 L46 36 L42 42 L22 42 Z" fill={base} opacity="0.5"/>
    {/* ember underline / heat */}
    <rect x="20" y="48" width="24" height="3" rx="1.5" fill={ember}/>
    <rect x="26" y="54" width="12" height="3" rx="1.5" fill={ember} opacity="0.6"/>
  </svg>
);

// PRIMARY MARK — switched to Bolt to match reference UI direction.
const MarkPrimary = MarkBolt;

// Wordmark — bold, tight, matches reference (Rx in accent + Forge in white)
const Wordmark = ({ color = "#f5efe6", accent = "#7c7cff", size = 32 }) => (
  <span style={{
    fontFamily: '"Space Grotesk", system-ui, sans-serif',
    fontWeight: 700,
    fontSize: size,
    letterSpacing: '-0.025em',
    color,
    lineHeight: 1,
  }}>
    <span style={{ color: accent }}>Rx</span>Forge
  </span>
);

// Lockup — bolt mark + wordmark side by side, matches reference
const Lockup = ({ Mark = MarkPrimary, color = "#f5efe6", ember = "#7c7cff", size = 40, gap = 10 }) => (
  <div style={{ display: 'inline-flex', alignItems: 'center', gap }}>
    <Mark size={size * 1.05} ember={ember} base={color} strokeW={3} />
    <Wordmark color={color} accent={ember} size={size * 0.95} />
  </div>
);

Object.assign(window, {
  MarkAnvilSpark,
  MarkHexForge,
  MarkLoopForge,
  MarkIngot,
  MarkBolt,
  MarkPrimary,
  Wordmark,
  Lockup,
});
