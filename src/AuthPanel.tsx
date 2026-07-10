import { useEffect, useState } from "react";
import { api } from "./api";
import type { AuthConfig, OAuth1SignatureMethod } from "./types";
import { defaultGrant, OAuth2Panel } from "./OAuth2Panel";
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

/** One-line human summary of an auth config, for the "this inherits X"
 * hint — never shows secret values, just the auth type/shape. */
export function describeAuth(auth: AuthConfig): string {
  switch (auth.type) {
    case "None":
      return "No auth";
    case "InheritFromParent":
      return "Inherit from parent";
    case "Basic":
      return "Basic Auth";
    case "Bearer":
      return "Bearer Token";
    case "ApiKey":
      return "API Key";
    case "OAuth1":
      return "OAuth 1.0";
    case "OAuth2":
      return "OAuth 2.0";
    case "AwsSigV4":
      return "AWS Signature V4";
    case "Digest":
      return "Digest Auth";
    case "Hawk":
      return "Hawk";
  }
}

function defaultAuth(type: AuthConfig["type"]): AuthConfig {
  switch (type) {
    case "InheritFromParent":
      return { type };
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
      return { type: "None" };
  }
}

interface Props {
  requestId: string;
  auth: AuthConfig;
  onChange: (auth: AuthConfig) => void;
  onCommit: (auth: AuthConfig) => void;
  variableNames: string[];
}

export function AuthPanel({
  requestId,
  auth,
  onChange,
  onCommit,
  variableNames,
}: Props) {
  const [effectiveAuth, setEffectiveAuth] = useState<AuthConfig | null>(null);

  useEffect(() => {
    if (auth.type !== "InheritFromParent") {
      setEffectiveAuth(null);
      return;
    }
    let cancelled = false;
    api
      .getEffectiveAuth(requestId)
      .then((resolved) => {
        if (!cancelled) setEffectiveAuth(resolved);
      })
      .catch(() => {
        if (!cancelled) setEffectiveAuth(null);
      });
    return () => {
      cancelled = true;
    };
  }, [requestId, auth.type]);

  function set(next: AuthConfig) {
    onChange(next);
    onCommit(next);
  }

  function field<K extends string>(key: K, value: string) {
    set({ ...auth, [key]: value } as AuthConfig);
  }

  return (
    <section className="panel">
      <h3>Authentication</h3>
      <div className="oauth2-field auth-type-field stacked-field">
        <label>Auth Type</label>
        <select
          value={auth.type}
          onChange={(e) =>
            set(defaultAuth(e.target.value as AuthConfig["type"]))
          }
        >
          {AUTH_TYPES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
      </div>

      {auth.type === "InheritFromParent" && (
        <p className="hint-text">
          {effectiveAuth
            ? `This request uses "${describeAuth(effectiveAuth)}" from a parent folder or the workspace.`
            : "This request has no auth configured on it or any parent folder/workspace — no Authorization is sent."}
        </p>
      )}

      {auth.type === "Basic" && (
        <div className="oauth2-panel">
          <div className="oauth2-field">
            <label>Username</label>
            <VarInput
              value={auth.username}
              onChange={(v) => field("username", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Password</label>
            <VarInput
              type="password"
              value={auth.password}
              onChange={(v) => field("password", v)}
              variableNames={variableNames}
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
              variableNames={variableNames}
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
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Value</label>
            <VarInput
              type="password"
              value={auth.value}
              onChange={(v) => field("value", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Add to</label>
            <select
              value={auth.location}
              onChange={(e) => field("location", e.target.value)}
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
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Password</label>
            <VarInput
              type="password"
              value={auth.password}
              onChange={(v) => field("password", v)}
              variableNames={variableNames}
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
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Secret Key</label>
            <VarInput
              type="password"
              value={auth.secret_key}
              onChange={(v) => field("secret_key", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Region</label>
            <VarInput
              placeholder="e.g. us-east-1"
              value={auth.region}
              onChange={(v) => field("region", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Service</label>
            <VarInput
              placeholder="e.g. execute-api"
              value={auth.service}
              onChange={(v) => field("service", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Session Token (optional)</label>
            <VarInput
              type="password"
              value={auth.session_token ?? ""}
              onChange={(v) => field("session_token", v)}
              variableNames={variableNames}
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
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Key</label>
            <VarInput
              type="password"
              value={auth.key}
              onChange={(v) => field("key", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Algorithm</label>
            <select
              value={auth.algorithm}
              onChange={(e) => field("algorithm", e.target.value)}
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
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Consumer Secret</label>
            <VarInput
              type="password"
              value={auth.consumer_secret}
              onChange={(v) => field("consumer_secret", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Token (optional)</label>
            <VarInput
              value={auth.token ?? ""}
              onChange={(v) => field("token", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Token Secret (optional)</label>
            <VarInput
              type="password"
              value={auth.token_secret ?? ""}
              onChange={(v) => field("token_secret", v)}
              variableNames={variableNames}
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
        <OAuth2Panel
          requestId={requestId}
          grant={auth.grant}
          onChange={(grant) => onChange({ type: "OAuth2", grant })}
          onCommit={(grant) => onCommit({ type: "OAuth2", grant })}
          variableNames={variableNames}
        />
      )}
    </section>
  );
}
