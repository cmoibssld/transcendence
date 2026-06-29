import "../styles/Rules.css";

const RULES = [
  {
    title: "Objective",
    body: "Beat the dealer by getting a hand value as close to 21 as possible without going over. Going over 21 is a bust — an automatic loss.",
  },
  {
    title: "Card values",
    body: "Number cards (2–10) are worth their face value. Face cards (Jack, Queen, King) are worth 10. Aces are worth 11, or 1 if 11 would cause a bust.",
  },
  {
    title: "Blackjack",
    body: "An Ace + a 10-value card on the initial deal is a Blackjack. It pays 3:2 (your bet × 1.5 profit) unless the dealer also has Blackjack — in which case it's a push.",
  },
  {
    title: "Hit or Stand",
    body: "After being dealt two cards, you may Hit (take another card) or Stand (keep your current hand). You can hit as many times as you like until you stand or bust.",
  },
  {
    title: "Dealer rules",
    body: "The dealer reveals their hidden card after all players have stood. The dealer must hit on any hand worth 16 or less, and must stand on 17 or more.",
  },
  {
    title: "Payouts",
    items: [
      { label: "Blackjack (3:2)", value: "Bet × 2.5 returned (stake + 1.5× profit)" },
      { label: "Normal win (1:1)", value: "Bet × 2 returned (stake + 1× profit)" },
      { label: "Push (tie)",       value: "Bet returned — no profit, no loss" },
      { label: "Loss",             value: "Bet forfeited" },
    ],
  },
  {
    title: "Betting",
    body: "Place your bet before cards are dealt. The bet is deducted immediately. Winnings (if any) are credited when the round ends.",
  },
  {
    title: "Multiple players",
    body: "Up to 3 players can join the same table. Each player acts independently. The dealer plays against everyone at once.",
  },
];

export default function Rules() {
  return (
    <div className="page-shell">
      <section className="page-card rules-page">
        <h1 className="page-title" style={{ marginTop: 12 }}>Rules</h1>
        <p className="page-copy" style={{ marginTop: 8 }}>
          Everything you need to know before sitting at the table.
        </p>

        <div className="rules-list">
          {RULES.map((rule, i) => (
            <div className="rule-item" key={i}>
              <div className="rule-number">{String(i + 1).padStart(2, "0")}</div>
              <div className="rule-body">
                <h3 className="rule-title">{rule.title}</h3>
                {rule.body && <p className="rule-text">{rule.body}</p>}
                {rule.items && (
                  <ul className="rule-payout-list">
                    {rule.items.map((it, j) => (
                      <li key={j}>
                        <span className="payout-label">{it.label}</span>
                        <span className="payout-value">{it.value}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
