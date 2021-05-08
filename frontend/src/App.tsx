import { Grid } from "@material-ui/core";
import React, { useState } from "react";
import { TwitterUser, useRemoveCandidates } from "./api";
import { UserCard } from "./UserCard";

const compareUsers = (a: TwitterUser, b: TwitterUser) => {
  if (a.friends_count === 0) {
    return -1;
  }
  if (b.friends_count === 0) {
    return 1;
  }

  const aLast = a.status ? Date.parse(a.status.created_at) : 0;
  const bLast = b.status ? Date.parse(b.status.created_at) : 0;
  return aLast - bLast;
};

const App = () => {
  const [confirmed, setConfirmed] = useState<number[]>([]);
  const removeUser = (userId: number) => {
    const next = [...confirmed];
    next.push(userId);
    setConfirmed(next);
  };

  const users = useRemoveCandidates() ?? [];
  const rows = [[]] as TwitterUser[][];
  users
    .sort(compareUsers)
    .filter((user) => !confirmed.includes(user.id))
    .forEach((user) => {
      if (rows[rows.length - 1].length === 6) {
        rows.push([user]);
      } else {
        rows[rows.length - 1].push(user);
      }
    });

  return (
    <div>
      {rows.map((row, i) => (
        <Grid key={i} container spacing={3}>
          {row.map((user) => (
            <UserCard
              key={user.screen_name}
              user={user}
              removeUser={removeUser}
            />
          ))}
        </Grid>
      ))}
    </div>
  );
};

export default App;
