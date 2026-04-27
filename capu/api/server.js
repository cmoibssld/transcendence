import express from "express";
import cors from "cors";
import dotenv from "dotenv";
import { Pool } from "pg";
import fs from "node:fs";
import https from "node:https";

dotenv.config();

const app = express();
const port = Number(process.env.PORT || 3001);
const tlsKeyPath = process.env.HTTPS_KEY_PATH || "./certs/dev-key.pem";
const tlsCertPath = process.env.HTTPS_CERT_PATH || "./certs/dev-cert.pem";

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

app.use(
  cors({
    origin: process.env.FRONTEND_ORIGIN || "http://localhost:5173",
  })
);
app.use(express.json());

app.get("/health", async (_req, res) => {
  try {
    await pool.query("SELECT 1");
    res.json({ ok: true });
  } catch (error) {
    res.status(500).json({ ok: false, message: String(error) });
  }
});

app.get("/api/players", async (_req, res) => {
  try {
    const { rows } = await pool.query(
      "SELECT id, username, email, money, avatar FROM users ORDER BY id ASC LIMIT 50"
    );
    res.json(rows);
  } catch (error) {
    res.status(500).json({ message: "Failed to fetch players", error: String(error) });
  }
});

app.get("/api/players/:id", async (req, res) => {
  try {
    const playerId = Number(req.params.id);
    if (!Number.isInteger(playerId) || playerId <= 0) {
      return res.status(400).json({ message: "Invalid player id" });
    }

    const { rows } = await pool.query(
      "SELECT id, username, email, money, avatar FROM users WHERE id = $1",
      [playerId]
    );

    if (rows.length === 0) {
      return res.status(404).json({ message: "Player not found" });
    }

    return res.json(rows[0]);
  } catch (error) {
    return res.status(500).json({ message: "Failed to fetch player", error: String(error) });
  }
});

app.patch("/api/players/:id", async (req, res) => {
  try {
    const playerId = Number(req.params.id);
    const { money, avatar } = req.body;

    if (!Number.isInteger(playerId) || playerId <= 0) {
      return res.status(400).json({ message: "Invalid player id" });
    }

    if (money === undefined && avatar === undefined) {
      return res.status(400).json({ message: "Provide money and/or avatar" });
    }

    const { rows } = await pool.query(
      `UPDATE users
       SET money = COALESCE($1, money),
           avatar = COALESCE($2, avatar)
       WHERE id = $3
       RETURNING id, username, email, money, avatar`,
      [money ?? null, avatar ?? null, playerId]
    );

    if (rows.length === 0) {
      return res.status(404).json({ message: "Player not found" });
    }

    return res.json(rows[0]);
  } catch (error) {
    return res.status(500).json({ message: "Failed to update player", error: String(error) });
  }
});

const httpsOptions = {
  key: fs.readFileSync(tlsKeyPath),
  cert: fs.readFileSync(tlsCertPath),
};

https.createServer(httpsOptions, app).listen(port, () => {
  console.log(`Player API running on https://localhost:${port}`);
});
