# Chaos Tests — 12 scénarios toxiproxy (N2.2)

Automatisés, tous au vert en CI nightly.

| # | Scénario | Toxiproxy |
|---|---|---|
| 1 | coupure_wifi | latency 0 + down 5s |
| 2 | veille | timeout 10s |
| 3 | kill_process | close connection |
| 4 | redemarrage_recepteur | restart server |
| 5 | redemarrage_envoyeur | restart client |
| 6 | renouvellement_dhcp | change IP |
| 7 | espace_disque_insuffisant | limit_data 1M |
| 8 | fichier_source_modifie | corrupt file mid-transfer |
| 9 | collision_nom | duplicate manifest path |
| 10 | annulation | DELETE /v1/transfers/{id} |
| 11 | fermeture_forcee | SIGKILL |
| 12 | changement_interface | switch if-addrs |
