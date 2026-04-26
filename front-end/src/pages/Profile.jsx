import { useState } from "react";
import "../styles/Profile.css";

import JB from "../assets/JB.jpg";
import Aloun from "../assets/Aloun.jpg";
import Alexis from "../assets/Alexis.jpg";
import Adrien from "../assets/Adrien.jpg";

const Profile = () => {

  const playerId = "BJX-4207";

  const avatarList = [JB, Aloun, Alexis, Adrien];

  const [showAvatars, setShowAvatars] = useState(false);

  const [profile, setProfile] = useState({
    firstName: "John",
    lastName: "Doe",
    username: "johndoe",
    email: "john@mail.com",
    country: "France",
    city: "Paris",
    gains: "$25,400",
    avatar: JB
  });

  const handleChange = (e) => {
    setProfile({
      ...profile,
      [e.target.name]: e.target.value
    });
  };

  const selectAvatar = (img) => {
    setProfile({
      ...profile,
      avatar: img
    });
  };

  return (
    <div className="profile-page">

      <div className="profile-container">

        {/* HEADER */}
        <div className="profile-header">

          <img src={profile.avatar} className="profile-avatar" />

          <div className="profile-info">

            <h2>{profile.firstName} {profile.lastName}</h2>

            <p>@{profile.username}</p>

            <p className="profile-id">ID: {playerId}</p>

            <button
              className="avatar-btn"
              onClick={() => setShowAvatars(true)}
            >
              Change avatar
            </button>

          </div>

        </div>

        {/* FORM */}
        <div className="profile-form">

          <div className="field">
            <label>Prénom</label>
            <input name="firstName" value={profile.firstName} onChange={handleChange}/>
          </div>

          <div className="field">
            <label>Nom</label>
            <input name="lastName" value={profile.lastName} onChange={handleChange}/>
          </div>

          <div className="field">
            <label>Username</label>
            <input name="username" value={profile.username} onChange={handleChange}/>
          </div>

          <div className="field">
            <label>Email</label>
            <input name="email" value={profile.email} onChange={handleChange}/>
          </div>

          <div className="field">
            <label>Pays</label>
            <input name="country" value={profile.country} onChange={handleChange}/>
          </div>

          <div className="field">
            <label>Ville</label>
            <input name="city" value={profile.city} onChange={handleChange}/>
          </div>

          <button className="save-btn">
            Save profile
          </button>

        </div>

      </div>

      {/* AVATAR MODAL */}
      {showAvatars && (
        <div className="avatar-modal">

          <div className="avatar-box">

            <h3>Choose your avatar</h3>

            <div className="avatar-grid">

              {avatarList.map((img, i) => (
                <img
                  key={i}
                  src={img}
                  className={`avatar-choice ${profile.avatar === img ? "active" : ""}`}
                  onClick={() => selectAvatar(img)}
                />
              ))}

            </div>

            <button
              className="confirm-btn"
              onClick={() => setShowAvatars(false)}
            >
              Confirm
            </button>

          </div>

        </div>
      )}

    </div>
  );
};

export default Profile;