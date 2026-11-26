# Working with other projects and conversations

Assistants in different projects or apps **can't talk to each other live**. Each conversation only sees its own messages. They can still work together by **reading and writing shared files**, and this wiki is set up for that.

## The Inbox window in the app

Click the **envelope** in the top bar, or choose **Help → Inbox**. An orange number on the envelope shows how many messages are still open.

- **Open / Handled / Outbox** list the messages. Each one is coloured by its status: open, in progress, needs answer or done.
- **Write a message** sends a message to the builder (it goes into `INBOX.md`) or to other projects (it goes into `OUTBOX.md`).
- **Copy note for another conversation** copies the note below, ready to paste into the other project.
- **Show inbox folder** opens the folder where files go.

The window reads the files again when they change, so a message another conversation writes shows up without restarting the app.

## How a message travels

1. **The other conversation → the builder.** The other assistant adds a message to this wiki's inbox:
   `/Users/amin/Documents/Craft Anything/vensimple/app/wiki/INBOX.md`
   It puts any files in `/Users/amin/Documents/Craft Anything/vensimple/app/wiki/inbox/`.
2. **The builder reads it** at the start of its next round, and replies under the message (**Reply (date):**).
3. **The builder → the other conversation.** The builder writes in [OUTBOX.md](OUTBOX.md). The other assistant reads it there. The builder may only change files inside `app/`, so it can't write into the other project's folder.
4. **Amin passes the word along.** Tell each conversation something like "check the Dynamist inbox" or "check the outbox". Neither one can see the other's new messages until it's asked to look.

## What to tell the other conversation

Paste this into the other project:

> Dynamist keeps a shared wiki at `/Users/amin/Documents/Craft Anything/vensimple/app/wiki`.
> Read `README.md` and `Spreadsheet-Template.md` first.
> To ask the Dynamist builder for something, add a message at the end of *Open messages* in `INBOX.md` (use the block shown there) and put files in `inbox/`.
> Read the builder's answers in `OUTBOX.md` and under your message in `INBOX.md`.

## What the builder can do with another project

- **Read it**, if Amin gives the full path to its folder or a file in it (for example a workbook the other assistant made).
- **Not change it.** Answers and results stay in this wiki and in the app.
