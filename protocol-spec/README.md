# Protocol Spec — RivaldSend v1

Spécification publique versionnée du protocole de transfert.

Voir `RivaldSend-Specification-v2.1.md` §4.

Endpoints :
- POST /v1/negotiate
- POST /v1/transfers
- PUT /v1/transfers/{id}/chunks?offset=N
- POST /v1/transfers/{id}/resume
- POST /v1/transfers/{id}/complete
