import {
  Avatar,
  Card,
  CardActions,
  CardContent,
  CardHeader,
  CardMedia,
  Collapse,
  Grid,
  IconButton,
  makeStyles,
  Typography,
} from "@material-ui/core";
import { red } from "@material-ui/core/colors";
import React, { useState } from "react";
import { TwitterUser, useRemoveCandidates } from "./api";

const useStyles = makeStyles((theme) => ({
  root: {
    maxWidth: 345,
  },
  media: {
    height: 0,
    paddingTop: "56.25%", // 16:9
  },
  expand: {
    transform: "rotate(0deg)",
    marginLeft: "auto",
    transition: theme.transitions.create("transform", {
      duration: theme.transitions.duration.shortest,
    }),
  },
  expandOpen: {
    transform: "rotate(180deg)",
  },
  avatar: {
    backgroundColor: red[500],
  },
}));

const App = () => {
  const classes = useStyles();
  const users = useRemoveCandidates() ?? [];
  const rows = [[]] as TwitterUser[][];
  users.forEach((user) => {
    if (rows[rows.length - 1].length === 6) {
      rows.push([user]);
    } else {
      rows[rows.length - 1].push(user);
    }
  });
  console.log(rows);

  return (
    <div>
      {rows.map((row, i) => (
        <Grid key={i} container spacing={3}>
          {row.map((user) => (
            <Grid key={user.screen_name} item xs={2}>
              <Card className={classes.root}>
                <CardHeader
                  title={user.name}
                  subheader={`@${user.screen_name}`}
                />
                <CardMedia
                  className={classes.media}
                  image={user.profile_image_url.replace("_normal", "_bigger")}
                />
                <CardContent>
                  <Typography
                    variant="body2"
                    color="textSecondary"
                    component="p"
                  >
                    {user.description}
                  </Typography>
                  <Typography
                    variant="body2"
                    color="textSecondary"
                    component="p"
                  >
                    {`Following: ${user.friends_count}, Followers: ${user.followers_count}`}
                  </Typography>
                </CardContent>
              </Card>
            </Grid>
          ))}
        </Grid>
      ))}
    </div>
  );
};

export default App;
