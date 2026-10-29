import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";
import useBaseUrl from "@docusaurus/useBaseUrl";
import styles from "./index.module.css";

const features = [
  {
    icon: "🧭",
    title: "Five modes",
    text: "Algebra, Physics, Imaginary, Analysis and Electronics. The mode decides how letters are read, so a plain x stays a variable and a g can be a constant.",
  },
  {
    icon: "✍️",
    title: "Formula editor or LaTeX",
    text: "Write in a MathLive editor or straight in LaTeX with a live preview, and switch between the two at any time without losing the input.",
  },
  {
    icon: "🧮",
    title: "Exact calculation",
    text: "Arbitrary precision numbers, units and dimensions that are checked and converted, plus equations, derivatives, integrals, limits and sums.",
  },
  {
    icon: "🪜",
    title: "The working, not just the result",
    text: "Five levels of step detail from the bare result to every intermediate line. Copy a result, or the whole calculation as an align* block.",
  },
  {
    icon: "📈",
    title: "Curve sketching",
    text: "Enter f(x) and get domain, symmetry, zeros, extrema, inflection points, asymptotes and an antiderivative beside an interactive graph.",
  },
  {
    icon: "⚡",
    title: "Circuits",
    text: "Draw passive DC circuits in a schematic editor and solve them exactly, even backwards for one unknown such as a cable cross-section.",
  },
];

export default function Home() {
  return (
    <Layout title="Quantus" description="A calculator for physical quantities, with LaTeX in and LaTeX out">
      <header className={styles.hero}>
        <div className="container">
          <img className={styles.logo} src={useBaseUrl("/img/icon.svg")} alt="" width="112" height="112" />
          <h1 className={styles.title}>Quantus</h1>
          <p className={styles.tagline}>A calculator for physical quantities, with LaTeX in and LaTeX out</p>
          <pre className={styles.example}>
            {"F = m_1 \\cdot 9{,}81\\frac{m}{s^2}\n→  F = 19{,}62\\,\\mathrm{N}"}
          </pre>
          <div className={styles.buttons}>
            <Link className="button button--primary button--lg" to="/docs/">
              Read the docs
            </Link>
            <Link className="button button--secondary button--lg" href="https://github.com/iswearimmrtobi/quantus">
              GitHub
            </Link>
          </div>
        </div>
      </header>
      <main className="container">
        <section className={styles.features}>
          {features.map((f) => (
            <div key={f.title} className={styles.card}>
              <div className={styles.icon}>{f.icon}</div>
              <h3>{f.title}</h3>
              <p>{f.text}</p>
            </div>
          ))}
        </section>
      </main>
    </Layout>
  );
}
