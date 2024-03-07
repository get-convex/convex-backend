/**
 * Information about an authenticated user.
 *
 * The only fields guaranteed to be present are
 * {@link UserIdentity.tokenIdentifier} and {@link UserIdentity.issuer}. All
 * remaining fields may or may not be present depending on the information given
 * by the identity provider.
 *
 * See the [OpenID Connect specification](https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims)
 * for more information on these fields.
 *
 * @public
 */
export interface UserIdentity {
  /**
   * A stable and globally unique string for this identity (i.e. no other
   * user, even from a different identity provider, will have the same string.)
   */
  readonly tokenIdentifier: string;

  /**
   * Identifier for the end-user from the identity provider, not necessarily
   * unique across different providers.
   */
  readonly subject: string;

  /**
   * The hostname of the identity provider used to authenticate this user.
   */
  readonly issuer: string;
  readonly name?: string;
  readonly givenName?: string;
  readonly familyName?: string;
  readonly nickname?: string;
  readonly preferredUsername?: string;
  readonly profileUrl?: string;
  readonly pictureUrl?: string;
  readonly email?: string;
  readonly emailVerified?: boolean;
  readonly gender?: string;
  readonly birthday?: string;
  readonly timezone?: string;
  readonly language?: string;
  readonly phoneNumber?: string;
  readonly phoneNumberVerified?: boolean;
  readonly address?: string;
  readonly updatedAt?: string;
}

/**
 * An interface to access information about the currently authenticated user
 * within Convex query and mutation functions.
 *
 * @public
 */
export interface Auth {
  /**
   * Get details about the currently authenticated user.
   *
   * @returns A promise that resolves to a {@link UserIdentity} if the Convex
   * client was configured with a valid ID token and `null` otherwise.
   */
  getUserIdentity(): Promise<UserIdentity | null>;
}
