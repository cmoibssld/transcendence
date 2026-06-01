import "../styles/Legal.css"

const SECTIONS = [
  {
    title: "1. Données collectées",
    content: [
      {
        subtitle: "Données de compte",
        text: "Lors de l'inscription, nous collectons votre nom d'utilisateur, votre adresse e-mail et un mot de passe hashé (bcrypt). Si vous utilisez la connexion via GitHub, nous récupérons votre identifiant GitHub, votre pseudo public et votre avatar.",
      },
      {
        subtitle: "Données de jeu",
        text: "Chaque partie jouée génère un enregistrement contenant : la table concernée, le numéro de round, le résultat (victoire, défaite, push, blackjack), la mise, le gain et l'horodatage. Ces données constituent votre historique et alimentent les statistiques.",
      },
      {
        subtitle: "Données de session",
        text: "Un token JWT (accès, 15 min) et un refresh token (30 jours) sont émis à la connexion et stockés côté client. Un cookie de session sécurisé est utilisé pour le flux OAuth GitHub.",
      },
      {
        subtitle: "Données techniques",
        text: "Les logs serveur peuvent contenir votre adresse IP, l'user-agent de votre navigateur et les horodatages des requêtes. Ces données ne sont utilisées qu'à des fins de sécurité et de débogage.",
      },
    ],
  },
  {
    title: "2. Utilisation des données",
    content: [
      {
        text: "Vos données sont utilisées exclusivement pour :",
        list: [
          "Vous authentifier et sécuriser votre compte",
          "Gérer vos coins virtuels et votre solde",
          "Afficher votre historique de parties et vos statistiques",
          "Permettre la fonctionnalité d'ajout d'amis et de consultation de leurs stats",
          "Assurer la sécurité et le bon fonctionnement de la plateforme",
        ],
      },
    ],
  },
  {
    title: "3. Stockage et sécurité",
    content: [
      {
        text: "Toutes les données sont stockées dans des bases de données PostgreSQL hébergées sur des serveurs privés. Les communications entre votre navigateur et nos serveurs sont chiffrées via TLS (HTTPS). Les communications entre les micro-services sont également chiffrées en TLS. Les mots de passe ne sont jamais stockés en clair — uniquement leur hash bcrypt.",
      },
    ],
  },
  {
    title: "4. Partage des données",
    content: [
      {
        text: "Nous ne vendons, ne louons et ne partageons vos données personnelles avec aucun tiers. Les seules données visibles par d'autres utilisateurs sont votre nom d'utilisateur, votre avatar et vos statistiques de jeu (uniquement pour vos amis acceptés).",
      },
    ],
  },
  {
    title: "5. Conservation",
    content: [
      {
        text: "Vos données sont conservées tant que votre compte est actif. L'historique de parties est conservé indéfiniment pour alimenter les statistiques. Les refresh tokens expirés sont supprimés automatiquement après 30 jours d'inactivité.",
      },
    ],
  },
  {
    title: "6. Vos droits",
    content: [
      {
        text: "Vous pouvez à tout moment :",
        list: [
          "Consulter et modifier vos informations de profil depuis la page Profil",
          "Supprimer votre compte en contactant l'équipe (voir ci-dessous)",
          "Demander l'export de vos données de jeu",
        ],
      },
    ],
  },
  {
    title: "7. Cookies",
    content: [
      {
        text: "Nous utilisons un seul cookie de session (HttpOnly, Secure, SameSite=Lax) uniquement pour sécuriser le flux d'authentification OAuth GitHub. Aucun cookie de tracking ou publicitaire n'est utilisé.",
      },
    ],
  },
  {
    title: "8. Contact",
    content: [
      {
        text: "Pour toute question relative à vos données personnelles ou pour exercer vos droits, contactez l'équipe du projet à l'adresse indiquée dans le dépôt du projet.",
      },
    ],
  },
]

export default function PrivacyPolicy() {
  return (
    <div className="page-shell">
      <section className="page-card legal-page">
        <div className="legal-header">
          <span className="section-chip">♠ Légal</span>
          <h1 className="page-title">Politique de confidentialité</h1>
          <p className="page-copy">
            Dernière mise à jour : juin 2026. Cette politique décrit comment Luigi Jack collecte, utilise et protège vos données personnelles.
          </p>
        </div>

        <div className="legal-body">
          {SECTIONS.map((section, i) => (
            <div className="legal-section" key={i}>
              <h2 className="legal-section-title">{section.title}</h2>
              {section.content.map((block, j) => (
                <div key={j} className="legal-block">
                  {block.subtitle && (
                    <h3 className="legal-subtitle">{block.subtitle}</h3>
                  )}
                  {block.text && <p className="legal-text">{block.text}</p>}
                  {block.list && (
                    <ul className="legal-list">
                      {block.list.map((item, k) => (
                        <li key={k}>{item}</li>
                      ))}
                    </ul>
                  )}
                </div>
              ))}
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}
