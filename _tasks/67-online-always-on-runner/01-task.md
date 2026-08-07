my homelab is ssh root@ubuntu.lacny.me in ~/_dev/home.notavailable repo, you have passwordless ssh access.

this app can run as web-service. Task is to make it run 24/7 as one of the services there, so I can access it from web-interface and access it via API to add trips etc. Research and investigate this repo - what would it take?

We have a sqlite DB, invoices, settings stored in prod in various locations - they would need a common home (on the server somewhere). Currently we save some of it in gdrive folder, so it's accessible from multiple computers that have the gdrive mapped. The server deploy and web access will fix this issue.

Do the research, come up with a design to do it.