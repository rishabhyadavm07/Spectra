import { useEffect, useRef, useState } from "react";
import { Eye, EyeOff, Copy, Trash2 } from "lucide-react";
import { api } from "./api";
import type {
  AddAuthDataTo,
  ClientAuthentication,
  NamedOAuthToken,
  OAuth2ExtraParam,
  OAuth2Grant,
  OAuthStatus,
  ParamTarget,
} from "./types";
import { defaultOAuth2Options } from "./types";
import { VarInput } from "./VarInput";

const OAUTH2_GRANTS: {
  value: OAuth2Grant["type"];
  label: string;
  interactive: boolean;
}[] = [
  {
    value: "ClientCredentials",
    label: "Client Credentials",
    interactive: false,
  },
  { value: "Password", label: "Password Credentials", interactive: false },
  { value: "RefreshToken", label: "Refresh Token", interactive: false },
  {
    value: "AuthorizationCode",
    label: "Authorization Code",
    interactive: true,
  },
  {
    value: "AuthorizationCodePkce",
    label: "Authorization Code (With PKCE)",
    interactive: true,
  },
  { value: "DeviceCode", label: "Device Code", interactive: true },
  { value: "Implicit", label: "Implicit (unsupported)", interactive: true },
];

export function defaultGrant(type: OAuth2Grant["type"]): OAuth2Grant {
  const options = defaultOAuth2Options();
  switch (type) {
    case "ClientCredentials":
      return {
        type,
        client_id: "",
        client_secret: "",
        token_url: "",
        scope: null,
        options,
      };
    case "Password":
      return {
        type,
        client_id: "",
        client_secret: null,
        token_url: "",
        username: "",
        password: "",
        scope: null,
        options,
      };
    case "RefreshToken":
      return {
        type,
        client_id: "",
        client_secret: null,
        token_url: "",
        refresh_token: "",
        options,
      };
    case "AuthorizationCode":
      return {
        type,
        client_id: "",
        client_secret: null,
        auth_url: "",
        token_url: "",
        redirect_uri: "spectra://oauth/callback",
        scope: null,
        options,
      };
    case "AuthorizationCodePkce":
      return {
        type,
        client_id: "",
        auth_url: "",
        token_url: "",
        redirect_uri: "spectra://oauth/callback",
        scope: null,
        options,
      };
    case "DeviceCode":
      return {
        type,
        client_id: "",
        device_auth_url: "",
        token_url: "",
        scope: null,
        options,
      };
    case "Implicit":
      return {
        type,
        client_id: "",
        auth_url: "",
        redirect_uri: "spectra://oauth/callback",
        scope: null,
        options,
      };
  }
}

interface Props {
  requestId: string;
  grant: OAuth2Grant;
  onChange: (grant: OAuth2Grant) => void;
  onCommit: (grant: OAuth2Grant) => void;
  variableNames: string[];
  isInherited?: boolean;
}

function formatExpiry(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  return `Expires at ${d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}${
    d.toDateString() === new Date().toDateString()
      ? " today"
      : ` on ${d.toLocaleDateString()}`
  }`;
}

export function OAuth2Panel({
  requestId,
  grant,
  onChange,
  onCommit,
  variableNames,
  isInherited,
}: Props) {
  const [tokens, setTokens] = useState<NamedOAuthToken[]>([]);
  const [tokenName, setTokenName] = useState("");
  const [showTokenValue, setShowTokenValue] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [oauthStatus, setOauthStatus] = useState<OAuthStatus | null>(null);
  const [flowBusy, setFlowBusy] = useState(false);
  const pollRef = useRef<number | null>(null);

  const grantMeta = OAUTH2_GRANTS.find((g) => g.value === grant.type);
  const currentTokenName = tokens[0]?.name ?? null;
  const currentToken = tokens.find((t) => t.name === currentTokenName) ?? null;

  useEffect(() => {
    refreshTokens();
    return () => {
      if (pollRef.current) window.clearInterval(pollRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [requestId]);

  async function refreshTokens() {
    try {
      setTokens(await api.listOAuthTokens(requestId));
    } catch {
      setTokens([]);
    }
  }

  function set(next: OAuth2Grant) {
    onChange(next);
    onCommit(next);
  }

  function field<K extends string>(key: K, value: string) {
    set({ ...grant, [key]: value } as OAuth2Grant);
  }

  function optionField<K extends string>(key: K, value: unknown) {
    set({
      ...grant,
      options: { ...grant.options, [key]: value },
    } as OAuth2Grant);
  }

  function addExtraParam(
    listKey: "token_request_params" | "refresh_request_params",
  ) {
    const next: OAuth2ExtraParam = { key: "", value: "", target: "body" };
    optionField(listKey, [...grant.options[listKey], next]);
  }

  function updateExtraParam(
    listKey: "token_request_params" | "refresh_request_params",
    index: number,
    patch: Partial<OAuth2ExtraParam>,
  ) {
    const list = grant.options[listKey].slice();
    list[index] = { ...list[index], ...patch };
    optionField(listKey, list);
  }

  function removeExtraParam(
    listKey: "token_request_params" | "refresh_request_params",
    index: number,
  ) {
    const list = grant.options[listKey].slice();
    list.splice(index, 1);
    optionField(listKey, list);
  }

  async function startInteractiveFlow() {
    setFlowBusy(true);
    setOauthStatus(null);
    try {
      await api.startOAuthFlow(requestId, tokenName.trim() || undefined);
      setTokenName("");
      pollRef.current = window.setInterval(async () => {
        const status = await api.getOAuthStatus(requestId);
        setOauthStatus(status);
        if (status.status === "Complete" || status.status === "Failed") {
          if (pollRef.current) window.clearInterval(pollRef.current);
          setFlowBusy(false);
          if (status.status === "Complete") refreshTokens();
        }
      }, 1500);
    } catch (e) {
      setOauthStatus({ status: "Failed", error: String(e) });
      setFlowBusy(false);
    }
  }

  async function getNewAccessToken() {
    setFlowBusy(true);
    setOauthStatus(null);
    try {
      await api.fetchOAuthToken(requestId, tokenName.trim() || undefined);
      setTokenName("");
      await refreshTokens();
    } catch (e) {
      setOauthStatus({ status: "Failed", error: String(e) });
    } finally {
      setFlowBusy(false);
    }
  }

  async function handleSelectToken(name: string) {
    await api.selectOAuthToken(requestId, name);
    await refreshTokens();
  }

  async function handleDeleteToken(name: string) {
    await api.deleteOAuthToken(requestId, name);
    await refreshTokens();
  }

  async function handleManualRefresh() {
    setFlowBusy(true);
    setOauthStatus(null);
    try {
      await api.refreshOAuthToken(requestId);
      await refreshTokens();
    } catch (e) {
      setOauthStatus({ status: "Failed", error: String(e) });
    } finally {
      setFlowBusy(false);
    }
  }

  return (
    <div className="oauth2-panel">
      <p className="hint-text oauth2-scope-note">
        This authorization method will be used for this request. Configure the
        grant below, then use <strong>Get New Access Token</strong> (or
        Authorize, for interactive grants) to fetch a token.
      </p>

      <div className="oauth2-field">
        <label>Grant Type</label>
        <select
          value={grant.type}
          disabled={isInherited}
          onChange={(e) =>
            set(defaultGrant(e.target.value as OAuth2Grant["type"]))
          }
        >
          {OAUTH2_GRANTS.map((g) => (
            <option
              key={g.value}
              value={g.value}
              disabled={g.value === "Implicit"}
            >
              {g.label}
            </option>
          ))}
        </select>
      </div>

      {tokens.length > 0 && (
        <>
          <div className="oauth2-section-label">Current Token</div>
          <div className="oauth2-field">
            <label>Token</label>
            <select
              value={currentTokenName ?? ""}
              onChange={(e) => handleSelectToken(e.target.value)}
            >
              {tokens.map((t) => (
                <option key={t.name} value={t.name}>
                  {t.name}
                </option>
              ))}
            </select>
          </div>
          {currentToken && (
            <>
              <div className="oauth2-field">
                <div className="oauth2-token-value-row">
                  {showTokenValue ? (
                    <textarea
                      className="oauth2-token-value-textarea"
                      readOnly
                      rows={6}
                      value={currentToken.token.access_token}
                    />
                  ) : (
                    <input
                      className="oauth2-token-value"
                      readOnly
                      type="password"
                      value={currentToken.token.access_token}
                    />
                  )}
                  <button
                    type="button"
                    onClick={() => setShowTokenValue((v) => !v)}
                    title="Show/hide token"
                    className="icon-button"
                  >
                    {showTokenValue ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                  <button
                    type="button"
                    onClick={() => navigator.clipboard.writeText(currentToken.token.access_token)}
                    title="Copy token"
                    className="icon-button"
                  >
                    <Copy size={16} />
                  </button>
                  <button
                    type="button"
                    onClick={() => handleDeleteToken(currentToken.name)}
                    title="Delete this token"
                    className="icon-button"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
                {currentToken.token.expires_at && (
                  <p className="hint-text">
                    {formatExpiry(currentToken.token.expires_at)}
                  </p>
                )}
                {currentToken.token.refresh_token && (
                  <button
                    type="button"
                    onClick={handleManualRefresh}
                    disabled={flowBusy}
                    style={{ marginTop: "0.5em" }}
                  >
                    Refresh Access Token
                  </button>
                )}
              </div>
            </>
          )}
        </>
      )}

      <div className="oauth2-section-label">Configure New Token</div>

      <div className="oauth2-field">
        <label>Token Name</label>
        <VarInput
          placeholder="Enter a token name…"
          value={tokenName}
          onChange={setTokenName}
          variableNames={variableNames}
        />
      </div>

      {!isInherited && (
        <>
          <div className="oauth2-field">
        <label>Client ID</label>
        <VarInput
          placeholder="Client ID"
          value={grant.client_id}
          onChange={(v) => field("client_id", v)}
          variableNames={variableNames}
        />
      </div>

      {"client_secret" in grant && (
        <div className="oauth2-field">
          <label>Client Secret</label>
          <VarInput
            placeholder="Client Secret"
            type="password"
            value={grant.client_secret ?? ""}
            onChange={(v) => field("client_secret", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      {"auth_url" in grant && (
        <div className="oauth2-field">
          <label>Authorization URL</label>
          <VarInput
            placeholder="Authorization URL"
            value={grant.auth_url}
            onChange={(v) => field("auth_url", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      {"device_auth_url" in grant && (
        <div className="oauth2-field">
          <label>Device Authorization URL</label>
          <VarInput
            placeholder="Device Authorization URL"
            value={grant.device_auth_url}
            onChange={(v) => field("device_auth_url", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      {"token_url" in grant && (
        <div className="oauth2-field">
          <label>Access Token URL</label>
          <VarInput
            placeholder="Access Token URL"
            value={grant.token_url}
            onChange={(v) => field("token_url", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      {"redirect_uri" in grant && (
        <div className="oauth2-field">
          <label>Redirect URI (loopback)</label>
          <VarInput
            placeholder="Redirect URI"
            value={grant.redirect_uri}
            onChange={(v) => field("redirect_uri", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      {grant.type === "Password" && (
        <>
          <div className="oauth2-field">
            <label>Username</label>
            <VarInput
              placeholder="Username"
              value={grant.username}
              onChange={(v) => field("username", v)}
              variableNames={variableNames}
            />
          </div>
          <div className="oauth2-field">
            <label>Password</label>
            <VarInput
              placeholder="Password"
              type="password"
              value={grant.password}
              onChange={(v) => field("password", v)}
              variableNames={variableNames}
            />
          </div>
        </>
      )}

      {grant.type === "RefreshToken" && (
        <div className="oauth2-field">
          <label>Refresh Token</label>
          <VarInput
            placeholder="Refresh Token"
            type="password"
            value={grant.refresh_token}
            onChange={(v) => field("refresh_token", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      {"scope" in grant && (
        <div className="oauth2-field">
          <label>Scope</label>
          <VarInput
            placeholder="e.g. read:org"
            value={grant.scope ?? ""}
            onChange={(v) => field("scope", v)}
            variableNames={variableNames}
          />
        </div>
      )}

      <div className="oauth2-field">
        <label>Client Authentication</label>
        <select
          value={grant.options.client_authentication}
          onChange={(e) =>
            optionField(
              "client_authentication",
              e.target.value as ClientAuthentication,
            )
          }
        >
          <option value="send_as_basic_auth_header">
            Send as Basic Auth header
          </option>
          <option value="send_in_body">Send client credentials in body</option>
        </select>
      </div>

      <div className="oauth2-field">
        <label>Add auth data to</label>
        <select
          value={grant.options.add_to}
          onChange={(e) =>
            optionField("add_to", e.target.value as AddAuthDataTo)
          }
        >
          <option value="request_headers">Request Headers</option>
          <option value="query_params">Query Params</option>
        </select>
      </div>

      <div className="oauth2-field">
        <label>Header Prefix</label>
        <input
          placeholder="Bearer"
          value={grant.options.header_prefix}
          onChange={(e) => optionField("header_prefix", e.target.value)}
        />
      </div>

      <div className="oauth2-toggle-row">
        <label>
          <input
            type="checkbox"
            checked={grant.options.auto_refresh}
            onChange={(e) => optionField("auto_refresh", e.target.checked)}
          />
          Auto-refresh Token
        </label>
        <p className="hint-text">
          Your expired token will be auto-refreshed (using its refresh_token)
          before sending a request.
        </p>
      </div>

      <button
        type="button"
        className="oauth2-advanced-toggle"
        onClick={() => setAdvancedOpen((v) => !v)}
      >
        {advancedOpen ? "▾" : "▸"} Advanced
      </button>

      {advancedOpen && (
        <div className="oauth2-advanced-box">
          <p className="hint-text">
            Extra key/value pairs sent alongside the token/refresh request — as
            a form body field or a request header.
          </p>

          <ExtraParamList
            label="Token Request Params"
            params={grant.options.token_request_params}
            variableNames={variableNames}
            onAdd={() => addExtraParam("token_request_params")}
            onUpdate={(i, patch) =>
              updateExtraParam("token_request_params", i, patch)
            }
            onRemove={(i) => removeExtraParam("token_request_params", i)}
          />

          <ExtraParamList
            label="Refresh Request Params"
            params={grant.options.refresh_request_params}
            variableNames={variableNames}
            onAdd={() => addExtraParam("refresh_request_params")}
            onUpdate={(i, patch) =>
              updateExtraParam("refresh_request_params", i, patch)
            }
            onRemove={(i) => removeExtraParam("refresh_request_params", i)}
          />
        </div>
      )}
      </>
      )}

      {grantMeta?.interactive && grant.type !== "Implicit" && (
        <div className="oauth-flow-box">
          <div style={{ display: "flex", gap: "0.5em", alignItems: "center" }}>
            <button onClick={startInteractiveFlow} disabled={flowBusy} style={{ flex: 1 }}>
              {flowBusy ? "Waiting for authorization…" : "Get New Access Token"}
            </button>
            {flowBusy && (
              <button
                onClick={async () => {
                  try {
                    await api.cancelOAuthFlow(requestId);
                  } catch {}
                  setFlowBusy(false);
                  setOauthStatus({ status: "Failed", error: "Cancelled by user." });
                }}
                style={{ flex: 0, padding: "0 1em" }}
                title="Cancel Authorization"
              >
                Cancel
              </button>
            )}
          </div>
          {oauthStatus?.status === "Pending" &&
            oauthStatus.user_action.kind === "DeviceCode" && (
              <p>
                Go to{" "}
                <strong>{oauthStatus.user_action.verification_url}</strong> and
                enter code <strong>{oauthStatus.user_action.user_code}</strong>
              </p>
            )}
          {oauthStatus?.status === "Pending" &&
            oauthStatus.user_action.kind === "Browser" && (
              <p>Complete sign-in in the browser window that opened.</p>
            )}
          {oauthStatus?.status === "Complete" && (
            <p className="status-ok">Authorized. Token acquired.</p>
          )}
          {oauthStatus?.status === "Failed" && (
            <p className="status-err">{oauthStatus.error}</p>
          )}
        </div>
      )}

      {grant.type === "Implicit" && (
        <p className="error-box">
          Implicit grant returns the token in a URL fragment a loopback server
          can't observe. Use Authorization Code (With PKCE) instead.
        </p>
      )}

      {!grantMeta?.interactive && (
        <div className="oauth-flow-box">
          <button onClick={getNewAccessToken} disabled={flowBusy}>
            {flowBusy ? "Fetching…" : "Get New Access Token"}
          </button>
          {oauthStatus?.status === "Failed" && (
            <p className="status-err">{oauthStatus.error}</p>
          )}
        </div>
      )}
    </div>
  );
}

interface ExtraParamListProps {
  label: string;
  params: OAuth2ExtraParam[];
  variableNames: string[];
  onAdd: () => void;
  onUpdate: (index: number, patch: Partial<OAuth2ExtraParam>) => void;
  onRemove: (index: number) => void;
}

function ExtraParamList({
  label,
  params,
  variableNames,
  onAdd,
  onUpdate,
  onRemove,
}: ExtraParamListProps) {
  return (
    <div className="oauth2-extra-params">
      <div className="oauth2-extra-params-header">
        <span className="oauth2-section-label">{label}</span>
        <button
          type="button"
          className="oauth2-extra-param-add"
          onClick={onAdd}
        >
          + Add
        </button>
      </div>
      {params.length === 0 && <p className="hint-text">No extra params.</p>}
      {params.map((p, i) => (
        <div className="oauth2-extra-param-row" key={i}>
          <VarInput
            placeholder="Key"
            value={p.key}
            onChange={(v) => onUpdate(i, { key: v })}
            variableNames={variableNames}
          />
          <VarInput
            placeholder="Value"
            value={p.value}
            onChange={(v) => onUpdate(i, { value: v })}
            variableNames={variableNames}
          />
          <select
            value={p.target}
            onChange={(e) =>
              onUpdate(i, { target: e.target.value as ParamTarget })
            }
          >
            <option value="body">Body</option>
            <option value="header">Header</option>
          </select>
          <button
            type="button"
            className="oauth2-extra-param-remove"
            onClick={() => onRemove(i)}
            title="Remove"
          >
            🗑
          </button>
        </div>
      ))}
    </div>
  );
}
