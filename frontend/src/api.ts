import useSWR from "swr";

export interface TwitterUser {
  id: number;
  description: string;
  name: string;
  screen_name: string;
  profile_image_url: string;
  friends_count: number;
  followers_count: number;
  status: TwitterStatus | null;
  protected: boolean;
  statuses_count: number;
}

export interface TwitterStatus {
  created_at: string;
}

export const useRemoveCandidates = () => {
  const fetcher = (url: string) =>
    fetch(url)
      .then((response) => response.json())
      .then((response) => response as TwitterUser[]);
  return useSWR<TwitterUser[]>("/remove_candidates", fetcher, {
    revalidateOnFocus: false,
    revalidateOnReconnect: false,
  }).data;
};

export const postConfirmRemove = async (user_id: number) => {
  const response = await fetch("/remove_user", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ user_id }),
  });
  return await response.json();
};
