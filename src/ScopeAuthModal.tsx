import { useState } from "react";
import type {
  AuthConfig,
  HawkAlgorithm,
  OAuth1SignatureMethod,
  OAuth2Grant,
} from "./types";
import { defaultGrant } from "./OAuth2Panel";
import { VarInput } from "./VarInput";

const AUTH_TYPES: { value: AuthConfig["type"]; label: string }[] = [
  { value: "InheritFromParent", label: "Inherit from parent" },
  { value: "None", label: "None" },
  { value: "Basic", label: "Basic Auth" },
  { value: "Bearer", label: "Bearer Token" },
  { value: "ApiKey", label: "API Key" },
  { value: "OAuth1", label: "OAuth 1.0" },
  { value: "OAuth2", label: "OAuth 2.0" },
  { value: "AwsSigV4", label: "AWS Signature V4" },
  { value: "Digest", label: "Digest Auth" },
  { value: "Hawk", label: "Hawk" },
];

const OAUTH2_NONINTERACTIVE_GRANTS: {
  value: OAuth2Grant["type"];
  label: string;
}[] = [
  { value: "ClientCredentials", label: "Client Credentials" },
  { value: "Password", label: "Password Credentials" },
  { value: "RefreshToken", label: "Refresh Token" },
];

function defaultAuth(type: AuthConfig["type"]): AuthConfig {
  switch (type) {
    case "Basic":
      return { type, username: "", password: "" };
    case "Bearer":
      return { type, token: "" };
    case "ApiKey":
      return { type, key: "", value: "", location: "header" };
    case "OAuth1":
      return {
        type,
        consumer_key: "",
        consumer_secret: "",
        token: null,
        token_secret: null,
        signature_method: "hmac_sha1",
      };
    case "OAuth2":
      return { type, grant: defaultGrant("ClientCredentials") };
    case "AwsSigV4":
      return {
        type,
        access_key: "",
        secret_key: "",
        region: "us-east-1",
        service: "execute-api",
        session_token: null,
      };
    case "Digest":
      return { type, username: "", password: "" };
    case "Hawk":
      return { type, id: "", key: "", algorithm: "SHA256" };
    default:
      return { type: type as "None" | "InheritFromParent" };
  }
}

interface Props {
  /** "Workspace" doesn't offer "Inherit from parent" — it's the top of the
   * chain, so that option is filtered out for it. */
  scope: "workspace" | "folder";
  scopeName: string;
  initialAuth: AuthConfig;
  onSave: (auth: AuthConfig) => Promise<void>;
  onClose: () => void;
}

/** Auth editor for a workspace or folder — the "collection auth" every
 * request under it inherits by default unless overridden further down the
 * chain. Its OAuth2 support is narrower than the per-request AuthPanel:
 * only the non-interactive grants (Client Credentials/Password/Refresh
 * Token) are usable here, since those just POST to a token URL with no
 * request-specific context. Authorization Code/PKCE/Device Code need a
 * concrete request to run their browser/loopback flow against, so they're
 * shown but disabled with an explanatory note, same as how the per-request
 * panel already disables the unsupported Implicit grant.
 */
export function ScopeAuthModal({
  scope,
  scopeName,
  initialAuth,
  onSave,
  onClose,
}: Props) {
  const [auth, setAuth] = useState<AuthConfig>(initialAuth);
  const [saving, setSaving] = useState(false);

  const authTypes =
    scope === "workspace"
      ? AUTH_TYPES.filter((t) => t.value !== "InheritFromParent")
      : AUTH_TYPES;

  function field<K extends string>(key: K, value: string) {
    setAuth((prev) => ({ ...prev, [key]: value }) as AuthConfig);
  }

  function grantField<K extends string>(key: K, value: string) {
    if (auth.type !== "OAuth2") return;
    setAuth({ ...auth, grant: { ...auth.grant, [key]: value } as OAuth2Grant });
  }

  async function handleSave() {
    setSaving(true);
    try {
      await onSave(auth);
      onClose();
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="env-editor-backdrop" onClick={onClose}>
      <div className="env-editor" onClick={(e) => e.stopPropagation()}>
        <div className="env-editor-header">
          <span className="env-name-input import-title">
            {scope === "workspace" ? "Workspace" : "Folder"} Auth — {scopeName}
          </span>
        </div>

        <div className="env-editor-body scope-auth-body">
          <p className="hint-text oauth2-scope-note">
            This authorization method will be used for every request{" "}
            {scope === "workspace" ? "in this workspace" : "in this folder"}.
            You can override this by specifying auth on a sub-folder or on the
            request itself.
          </p>

          <div className="oauth2-field auth-type-field">
            <label>Auth Type</label>
            <select
              value={auth.type}
              onChange={(e) =>
                setAuth(defaultAuth(e.target.value as AuthConfig["type"]))
              }
            >
              {authTypes.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </div>

          {auth.type === "Basic" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Username</label>
                <VarInput
                  value={auth.username}
                  onChange={(v) => field("username", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Password</label>
                <VarInput
                  type="password"
                  value={auth.password}
                  onChange={(v) => field("password", v)}
                  variableNames={[]}
                />
              </div>
            </div>
          )}

          {auth.type === "Bearer" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Token</label>
                <VarInput
                  type="password"
                  value={auth.token}
                  onChange={(v) => field("token", v)}
                  variableNames={[]}
                />
              </div>
            </div>
          )}

          {auth.type === "ApiKey" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Key</label>
                <VarInput
                  value={auth.key}
                  onChange={(v) => field("key", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Value</label>
                <VarInput
                  type="password"
                  value={auth.value}
                  onChange={(v) => field("value", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Add to</label>
                <select
                  value={auth.location}
                  onChange={(e) =>
                    field(
                      "location",
                      e.target.value as "header" | "query" | "cookie",
                    )
                  }
                >
                  <option value="header">Header</option>
                  <option value="query">Query Param</option>
                  <option value="cookie">Cookie</option>
                </select>
              </div>
            </div>
          )}

          {auth.type === "Digest" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Username</label>
                <VarInput
                  value={auth.username}
                  onChange={(v) => field("username", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Password</label>
                <VarInput
                  type="password"
                  value={auth.password}
                  onChange={(v) => field("password", v)}
                  variableNames={[]}
                />
              </div>
            </div>
          )}

          {auth.type === "AwsSigV4" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Access Key</label>
                <VarInput
                  value={auth.access_key}
                  onChange={(v) => field("access_key", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Secret Key</label>
                <VarInput
                  type="password"
                  value={auth.secret_key}
                  onChange={(v) => field("secret_key", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Region</label>
                <VarInput
                  placeholder="e.g. us-east-1"
                  value={auth.region}
                  onChange={(v) => field("region", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Service</label>
                <VarInput
                  placeholder="e.g. execute-api"
                  value={auth.service}
                  onChange={(v) => field("service", v)}
                  variableNames={[]}
                />
              </div>
            </div>
          )}

          {auth.type === "Hawk" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Hawk ID</label>
                <VarInput
                  value={auth.id}
                  onChange={(v) => field("id", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Key</label>
                <VarInput
                  type="password"
                  value={auth.key}
                  onChange={(v) => field("key", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Algorithm</label>
                <select
                  value={auth.algorithm}
                  onChange={(e) =>
                    field("algorithm", e.target.value as HawkAlgorithm)
                  }
                >
                  <option value="SHA256">SHA256</option>
                  <option value="SHA1">SHA1</option>
                </select>
              </div>
            </div>
          )}

          {auth.type === "OAuth1" && (
            <div className="oauth2-panel">
              <div className="oauth2-field">
                <label>Consumer Key</label>
                <VarInput
                  value={auth.consumer_key}
                  onChange={(v) => field("consumer_key", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Consumer Secret</label>
                <VarInput
                  type="password"
                  value={auth.consumer_secret}
                  onChange={(v) => field("consumer_secret", v)}
                  variableNames={[]}
                />
              </div>
              <div className="oauth2-field">
                <label>Signature Method</label>
                <select
                  value={auth.signature_method}
                  onChange={(e) =>
                    field(
                      "signature_method",
                      e.target.value as OAuth1SignatureMethod,
                    )
                  }
                >
                  <option value="hmac_sha1">HMAC-SHA1</option>
                  <option value="hmac_sha256">HMAC-SHA256</option>
                  <option value="plain_text">PLAINTEXT</option>
                </select>
              </div>
            </div>
          )}

          {auth.type === "OAuth2" && (
            <div className="oauth2-panel">
              <p className="hint-text oauth2-scope-note">
                Only non-interactive grants (Client Credentials/Password/Refresh
                Token) are supported at the {scope} level — a token is fetched
                automatically on Send. Authorization Code/PKCE/Device Code need
                a specific request to run their browser flow against, so
                configure those on the request itself instead.
              </p>
              <div className="oauth2-field">
                <label>Grant Type</label>
                <select
                  value={
                    OAUTH2_NONINTERACTIVE_GRANTS.some(
                      (g) => g.value === auth.grant.type,
                    )
                      ? auth.grant.type
                      : "ClientCredentials"
                  }
                  onChange={(e) =>
                    setAuth({
                      type: "OAuth2",
                      grant: defaultGrant(
                        e.target.value as OAuth2Grant["type"],
                      ),
                    })
                  }
                >
                  {OAUTH2_NONINTERACTIVE_GRANTS.map((g) => (
                    <option key={g.value} value={g.value}>
                      {g.label}
                    </option>
                  ))}
                </select>
              </div>
              {"token_url" in auth.grant && (
                <div className="oauth2-field">
                  <label>Access Token URL</label>
                  <VarInput
                    value={auth.grant.token_url}
                    onChange={(v) => grantField("token_url", v)}
                    variableNames={[]}
                  />
                </div>
              )}
              <div className="oauth2-field">
                <label>Client ID</label>
                <VarInput
                  value={auth.grant.client_id}
                  onChange={(v) => grantField("client_id", v)}
                  variableNames={[]}
                />
              </div>
              {"client_secret" in auth.grant && (
                <div className="oauth2-field">
                  <label>Client Secret</label>
                  <VarInput
                    type="password"
                    value={auth.grant.client_secret ?? ""}
                    onChange={(v) => grantField("client_secret", v)}
                    variableNames={[]}
                  />
                </div>
              )}
              {auth.grant.type === "Password" && (
                <>
                  <div className="oauth2-field">
                    <label>Username</label>
                    <VarInput
                      value={auth.grant.username}
                      onChange={(v) => grantField("username", v)}
                      variableNames={[]}
                    />
                  </div>
                  <div className="oauth2-field">
                    <label>Password</label>
                    <VarInput
                      type="password"
                      value={auth.grant.password}
                      onChange={(v) => grantField("password", v)}
                      variableNames={[]}
                    />
                  </div>
                </>
              )}
              {auth.grant.type === "RefreshToken" && (
                <div className="oauth2-field">
                  <label>Refresh Token</label>
                  <VarInput
                    type="password"
                    value={auth.grant.refresh_token}
                    onChange={(v) => grantField("refresh_token", v)}
                    variableNames={[]}
                  />
                </div>
              )}
              {"scope" in auth.grant && (
                <div className="oauth2-field">
                  <label>Scope</label>
                  <VarInput
                    value={auth.grant.scope ?? ""}
                    onChange={(v) => grantField("scope", v)}
                    variableNames={[]}
                  />
                </div>
              )}
            </div>
          )}
        </div>

        <div className="env-editor-footer">
          <span />
          <div className="env-editor-footer-right">
            <button onClick={onClose}>Cancel</button>
            <button onClick={handleSave} disabled={saving}>
              {saving ? "Saving…" : "Save"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
