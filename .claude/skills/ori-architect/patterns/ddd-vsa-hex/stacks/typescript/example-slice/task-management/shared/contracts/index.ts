// Cross-feature contracts live here.
//
// Features must not import each other directly. When feature A needs to react
// to something feature B owns, declare the shared shape (event payload, port
// interface) in this folder and have both sides depend on the contract — not
// on each other's implementation.
//
// Empty by default; add contracts as cross-feature collaboration grows.
export {};
