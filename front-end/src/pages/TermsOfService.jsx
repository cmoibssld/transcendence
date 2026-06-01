import "../styles/Legal.css"

const SECTIONS = [
  {
    title: "1. Acceptation des conditions",
    content: [
      {
        text: "En accédant à Luigi Jack et en créant un compte, vous acceptez sans réserve les présentes Conditions Générales d'Utilisation (CGU). Si vous n'acceptez pas ces conditions, vous ne pouvez pas utiliser la plateforme.",
      },
    ],
  },
  {
    title: "2. Description du service",
    content: [
      {
        text: "Luigi Jack est une plateforme de jeu de Blackjack multijoueur à vocation éducative et de divertissement. Elle propose :",
        list: [
          "Des parties de Blackjack en temps réel contre d'autres joueurs",
          "Un système de coins virtuels (aucune valeur monétaire réelle)",
          "Un historique de parties et des statistiques personnelles",
          "Un système d'amis et un chat intégré",
        ],
      },
      {
        text: "Luigi Jack est un projet scolaire développé dans le cadre de l'école 42. Il n'implique aucun argent réel et n'est pas une plateforme de jeu d'argent au sens légal du terme.",
      },
    ],
  },
  {
    title: "3. Compte utilisateur",
    content: [
      {
        subtitle: "Création de compte",
        text: "Vous pouvez vous inscrire avec une adresse e-mail et un mot de passe, ou via votre compte GitHub. Vous êtes responsable de l'exactitude des informations fournies et de la confidentialité de vos identifiants.",
      },
      {
        subtitle: "Responsabilités",
        text: "Vous êtes seul responsable de toute activité effectuée depuis votre compte. En cas de compromission, vous devez en informer l'équipe immédiatement. Un seul compte par personne est autorisé.",
      },
      {
        subtitle: "Suspension et résiliation",
        text: "L'équipe se réserve le droit de suspendre ou supprimer tout compte en cas de violation des présentes CGU, de comportement abusif, ou de tentative de triche.",
      },
    ],
  },
  {
    title: "4. Règles de conduite",
    content: [
      {
        text: "En utilisant Luigi Jack, vous vous engagez à ne pas :",
        list: [
          "Tenter d'exploiter des failles techniques pour obtenir des avantages injustes",
          "Harceler, insulter ou menacer d'autres utilisateurs via le chat",
          "Usurper l'identité d'un autre joueur",
          "Automatiser votre jeu ou utiliser des bots",
          "Tenter d'accéder aux données d'autres utilisateurs",
          "Perturber le bon fonctionnement de la plateforme",
        ],
      },
    ],
  },
  {
    title: "5. Coins virtuels",
    content: [
      {
        text: "Les coins utilisés sur Luigi Jack sont strictement virtuels. Ils n'ont aucune valeur monétaire, ne peuvent pas être échangés contre de l'argent réel, et ne peuvent pas être transférés entre comptes. Leur solde peut être réinitialisé sans préavis en cas de triche avérée.",
      },
    ],
  },
  {
    title: "6. Propriété intellectuelle",
    content: [
      {
        text: "Le code source de Luigi Jack est mis à disposition dans le dépôt du projet conformément à sa licence. Les assets visuels, logos et designs sont la propriété de l'équipe du projet. Vous ne pouvez pas reproduire ou distribuer ces éléments sans autorisation.",
      },
    ],
  },
  {
    title: "7. Limitation de responsabilité",
    content: [
      {
        text: "Luigi Jack est fourni \"tel quel\", sans garantie de disponibilité continue ni d'absence de bugs. L'équipe ne pourra être tenue responsable de toute perte de données, interruption de service, ou dysfonctionnement. La plateforme étant un projet scolaire, des interruptions de maintenance peuvent survenir sans préavis.",
      },
    ],
  },
  {
    title: "8. Modifications des CGU",
    content: [
      {
        text: "L'équipe se réserve le droit de modifier ces CGU à tout moment. Les modifications entrent en vigueur dès leur publication sur cette page. La date de dernière mise à jour est indiquée en en-tête. L'utilisation continue du service après modification vaut acceptation des nouvelles conditions.",
      },
    ],
  },
  {
    title: "9. Droit applicable",
    content: [
      {
        text: "Les présentes CGU sont soumises au droit français. En cas de litige, les parties tenteront de trouver une résolution amiable avant tout recours judiciaire.",
      },
    ],
  },
  {
    title: "10. Contact",
    content: [
      {
        text: "Pour toute question relative aux présentes CGU, contactez l'équipe du projet à l'adresse indiquée dans le dépôt du projet.",
      },
    ],
  },
]

export default function TermsOfService() {
  return (
    <div className="page-shell">
      <section className="page-card legal-page">
        <div className="legal-header">
          <span className="section-chip">♠ Légal</span>
          <h1 className="page-title">Conditions Générales d'Utilisation</h1>
          <p className="page-copy">
            Dernière mise à jour : juin 2026. En utilisant Luigi Jack, vous acceptez les conditions décrites ci-dessous.
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
