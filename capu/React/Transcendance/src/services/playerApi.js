const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || "https://localhost:3001";

export async function fetchPlayerById(playerId) {
  const response = await fetch(`${API_BASE_URL}/api/players/${playerId}`, {
    method: "GET",
    headers: {
      Accept: "application/json",
    },
  });
  if (!response.ok) {
    throw new Error(`Cannot fetch player ${playerId}`);
  }
  return response.json();
}

export { API_BASE_URL };
