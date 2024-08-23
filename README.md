# memcal

`memcal` is an iCal compatible server. It accepts existing iCal urls and
responds with the same data. With one important distinction: it parses the
source data and stores them in a datastore.
This is useful for iCal feeds that remove events after they have passed.

> **Note**: Datastore is subject to change. Currently it's a sqlite file.
> But I'd like to explore options like `rocksdb` or `leveldb`.

## Pre-requisites

You will need `cargo` to build the project.
`sqlite3` is required to interact with the sqlite database.

### Create initial database

```bash
sqlite3 data/memcal.db
```

## Architecture

The primary interaction with `memcal` is through the REST API.
The API is a simple CRUD interface that allows to manage iCal feeds.

- `POST /feed` - Add a new iCal feed
- `GET /feed/:id` - Get a memorized iCal feed
- `DELETE /feed/:id` - Remove a memorized iCal feed

The syncing is independent of the API. It's a background process that runs
every 5 minutes. It fetches the iCal feeds and updates the datastore.

We also expose a web interface that allows to manage the feeds.

- You can add new feeds.
- Delete existing feeds.

There's no plans for authentication or authorization at the moment.

### Adding a new feed

To add a new feed you can use the API or the web interface.

```bash
curl -H "content-type: application/json" \
    -d '{"url": "https://example.com/feed.ics"}' \
    http://localhost:8080/feed
```

This will respond with the url of memorized feed.

```json
{
  "url": "http://localhost:8080/feed/<feed_id>",
  "manage_token": "<manage_token>",
  "manage_url": "http://localhost:8080/feed/feed_id/<manage_token>"
}
```

The `feed_id` is a short alphanumeric code that identifies the feed.
The `manage_token` is a token that allows to manage the feed. With this token
you can delete the feed.
The `manage_url` is the url that allows to delete the feed using the web UI.

### Getting a feed

To get a memorized feed you can use the feed url you got when adding the feed.

```bash
curl http://localhost:8080/feed/<feed_id>
```

This will respond with the iCal data that can be used in any iCal compatible
client.

### Deleting a feed

To delete a memorized feed you can use the feed url you got when adding the feed.

```bash
curl -X DELETE \
    -H "Authorization: Bearer <manage_token>" \
    http://localhost:8080/feed/<feed_id>
```

This will respond with a 204 status code if the feed was deleted successfully.

The web interface is available at `http://localhost:8080`.
You can add new feeds here.
It will redirect you to the feed management page at
`http://localhost:8080/feed/<feed_id>/<manage_token>`
This shows the iCal url and options to delete the feed.

In addition to the API and the web interface, you can also use the CLI tool

```bash
memcal add https://example.com/feed.ics
memcal download <feed_id> -o feed.ics
memcal delete <feed_id> # or memcal delete <feed_url>
```

## Future

- At some point it might be worth to explore url parameters that allow smart
  features like filtering, sorting, etc.
  eg. `?since=2020-01-01` or `?since=last_year`
