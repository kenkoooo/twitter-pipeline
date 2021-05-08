import {
  Button,
  Card,
  CardActions,
  CardContent,
  CardMedia,
  Grid,
  Link,
  makeStyles,
  Typography,
} from "@material-ui/core";
import { red } from "@material-ui/core/colors";
import React from "react";
import { postConfirmRemove, TwitterUser } from "./api";

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
interface Props {
  user: TwitterUser;
  removeUser: (userId: number) => void;
}

const formatTime = (milliSeconds: number) => {
  const days = milliSeconds / 1000 / 3600 / 24;
  if (days > 365) {
    return `${Math.floor(days / 365)} years`;
  } else if (days > 30) {
    return `${Math.floor(days / 30)} months`;
  } else {
    return `${Math.floor(days)} days`;
  }
};

export const UserCard = (props: Props) => {
  const { user } = props;
  const classes = useStyles();
  const lastPost = user.status ? Date.parse(user.status.created_at) : null;

  return (
    <Grid item xs={2}>
      <Card className={classes.root}>
        <CardMedia
          className={classes.media}
          image={user.profile_image_url.replace("_normal", "_bigger")}
        />
        <CardContent>
          <Typography variant="h5" component="h2">
            <Link
              href={`https://twitter.com/${user.screen_name}`}
              target="_blank"
              rel="noreferrer noopener"
            >
              {user.name}
              {user.protected ? <span>&#x1f512;</span> : <span />}
            </Link>
          </Typography>
          <Typography variant="body2" component="p" gutterBottom>
            {`@${user.screen_name}`}
          </Typography>
          <Typography variant="body2" color="textSecondary" component="p">
            {user.statuses_count} tweets
          </Typography>
          {lastPost && (
            <Typography variant="body2" color="textSecondary" component="p">
              {formatTime(Date.now() - lastPost)} ago
            </Typography>
          )}
          <Typography
            variant="body2"
            color="textSecondary"
            component="p"
            gutterBottom
          >
            {user.description}
          </Typography>
          <Typography variant="body2" component="p">
            {`Following: ${user.friends_count}, Followers: ${user.followers_count}`}
          </Typography>
        </CardContent>
        <CardActions>
          <Button
            variant="contained"
            color="primary"
            onClick={async () => {
              props.removeUser(user.id);
            }}
          >
            Allow
          </Button>
          <Button
            variant="contained"
            color="secondary"
            onClick={async () => {
              props.removeUser(user.id);
              await postConfirmRemove(user.id);
            }}
          >
            Remove
          </Button>
        </CardActions>
      </Card>
    </Grid>
  );
};
