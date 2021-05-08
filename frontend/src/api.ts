import useSWR from "swr";

export interface TwitterUser {
  description: string;
  name: string;
  screen_name: string;
  profile_image_url: string;
  friends_count: number;
  followers_count: number;
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
