import "../styles/Friends.css";

import Aloun from "../assets/Aloun.jpg";
import Alexis from "../assets/Alexis.jpg";
import Adrien from "../assets/Adrien.jpg";

const Friends = () => {
  const playerId = "BJX-4207";

  const friends = [
    { name: "Aloun", avatar: Aloun, online: true, gains: "$12,450" },
    { name: "Alexis", avatar: Alexis, online: true, gains: "$21,900" },
    { name: "Adrien", avatar: Adrien, online: false, gains: "$5,300" },
  ];

  return (
    <div className="page-shell">
      <section className="friends-page page-card">
        <div className="top-bar">
          <div>
            <span className="section-chip">Community</span>
            <h2 className="friends-label">Friends</h2>
          </div>

          <div className="search-bar">
            <input
              type="text"
              placeholder="Search for friends..."
            />
          </div>
        </div>

        <div className="friends-grid">
          {friends.length > 0 ? (
            friends.map((friend, index) => (
              <div className="friend-card" key={index}>
                <img
                  src={friend.avatar}
                  alt={friend.name}
                  className="friend-avatar"
                />

                <span className="friend-name">
                  {friend.name}
                </span>

                <span className="friend-gains">
                  Gains: {friend.gains}
                </span>

                <div className="friend-status">
                  <span
                    className={`dot ${
                      friend.online ? "online" : "offline"
                    }`}
                  ></span>

                  {friend.online ? "Online" : "Offline"}
                </div>
              </div>
            ))
          ) : (
            <div className="no-friends">
              <p>No friends yet.</p>
              <span>Search by ID to add your first friend.</span>
            </div>
          )}
        </div>
      </section>
    </div>
  );
};

export default Friends;