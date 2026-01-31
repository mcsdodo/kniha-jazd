/**
 * Application-wide constants
 *
 * This file contains all magic string constants used across the frontend.
 * Use these constants instead of raw string literals for type safety and consistency.
 */

// =============================================================================
// Vehicle Types
// =============================================================================

export const VEHICLE_TYPES = {
	ICE: 'Ice',
	BEV: 'Bev',
	PHEV: 'Phev'
} as const;
export type VehicleType = (typeof VEHICLE_TYPES)[keyof typeof VEHICLE_TYPES];

// =============================================================================
// Toast Notifications
// =============================================================================

export const TOAST_TYPES = {
	SUCCESS: 'success',
	ERROR: 'error',
	INFO: 'info'
} as const;
export type ToastType = (typeof TOAST_TYPES)[keyof typeof TOAST_TYPES];

// =============================================================================
// Receipt Status
// =============================================================================

export const RECEIPT_STATUS = {
	PENDING: 'Pending',
	PARSED: 'Parsed',
	NEEDS_REVIEW: 'NeedsReview',
	ASSIGNED: 'Assigned'
} as const;
export type ReceiptStatus = (typeof RECEIPT_STATUS)[keyof typeof RECEIPT_STATUS];

// =============================================================================
// Receipt Filters
// =============================================================================

export const RECEIPT_FILTERS = {
	ALL: 'all',
	UNASSIGNED: 'unassigned',
	NEEDS_REVIEW: 'needs_review'
} as const;
export type ReceiptFilter = (typeof RECEIPT_FILTERS)[keyof typeof RECEIPT_FILTERS];

export const RECEIPT_TYPE_FILTERS = {
	ALL: 'all',
	FUEL: 'fuel',
	OTHER: 'other'
} as const;
export type ReceiptTypeFilter = (typeof RECEIPT_TYPE_FILTERS)[keyof typeof RECEIPT_TYPE_FILTERS];

// =============================================================================
// Confidence Levels
// =============================================================================

export const CONFIDENCE_LEVELS = {
	HIGH: 'High',
	MEDIUM: 'Medium',
	LOW: 'Low',
	UNKNOWN: 'Unknown'
} as const;
export type ConfidenceLevel = (typeof CONFIDENCE_LEVELS)[keyof typeof CONFIDENCE_LEVELS];

// =============================================================================
// Theme Modes
// =============================================================================

export const THEME_MODES = {
	SYSTEM: 'system',
	LIGHT: 'light',
	DARK: 'dark'
} as const;
export type ThemeMode = (typeof THEME_MODES)[keyof typeof THEME_MODES];

// =============================================================================
// Backup/Update Steps
// =============================================================================

export const BACKUP_STEPS = {
	PENDING: 'pending',
	IN_PROGRESS: 'in-progress',
	DONE: 'done',
	FAILED: 'failed',
	SKIPPED: 'skipped'
} as const;
export type BackupStep = (typeof BACKUP_STEPS)[keyof typeof BACKUP_STEPS];

// =============================================================================
// Sort Options
// =============================================================================

export const SORT_COLUMNS = {
	MANUAL: 'manual',
	DATE: 'date'
} as const;
export type SortColumn = (typeof SORT_COLUMNS)[keyof typeof SORT_COLUMNS];

export const SORT_DIRECTIONS = {
	ASC: 'asc',
	DESC: 'desc'
} as const;
export type SortDirection = (typeof SORT_DIRECTIONS)[keyof typeof SORT_DIRECTIONS];

// =============================================================================
// Attachment Status
// =============================================================================

export const ATTACHMENT_STATUS = {
	EMPTY: 'empty',
	MATCHES: 'matches',
	DIFFERS: 'differs'
} as const;
export type AttachmentStatus = (typeof ATTACHMENT_STATUS)[keyof typeof ATTACHMENT_STATUS];

// =============================================================================
// Mismatch Reasons
// =============================================================================

export const MISMATCH_REASONS = {
	NONE: 'none',
	DATE: 'date',
	LITERS: 'liters',
	PRICE: 'price',
	LITERS_AND_PRICE: 'liters_and_price',
	DATE_AND_LITERS: 'date_and_liters',
	DATE_AND_PRICE: 'date_and_price',
	ALL: 'all'
} as const;
export type MismatchReason = (typeof MISMATCH_REASONS)[keyof typeof MISMATCH_REASONS];

// =============================================================================
// Currencies
// =============================================================================

export const CURRENCIES = {
	EUR: 'EUR',
	CZK: 'CZK',
	HUF: 'HUF',
	PLN: 'PLN'
} as const;
export type Currency = (typeof CURRENCIES)[keyof typeof CURRENCIES];
export const PRIMARY_CURRENCY = CURRENCIES.EUR;

// =============================================================================
// Locales
// =============================================================================

export const LOCALES = {
	SK: 'sk',
	EN: 'en'
} as const;
export type Locale = (typeof LOCALES)[keyof typeof LOCALES];

export const LOCALE_CODES = {
	SK: 'sk-SK',
	EN: 'en-US'
} as const;
export type LocaleCode = (typeof LOCALE_CODES)[keyof typeof LOCALE_CODES];

// =============================================================================
// Keyboard Keys
// =============================================================================

export const KEYBOARD_KEYS = {
	ESCAPE: 'Escape',
	ENTER: 'Enter',
	TAB: 'Tab',
	ARROW_DOWN: 'ArrowDown',
	ARROW_UP: 'ArrowUp'
} as const;

// =============================================================================
// Download Events (for updates)
// =============================================================================

export const DOWNLOAD_EVENTS = {
	STARTED: 'Started',
	PROGRESS: 'Progress',
	FINISHED: 'Finished'
} as const;
export type DownloadEvent = (typeof DOWNLOAD_EVENTS)[keyof typeof DOWNLOAD_EVENTS];

// =============================================================================
// Date Prefill Modes
// =============================================================================

export const DATE_PREFILL_MODES = {
	PREVIOUS: 'previous',
	TODAY: 'today'
} as const;
export type DatePrefillMode = (typeof DATE_PREFILL_MODES)[keyof typeof DATE_PREFILL_MODES];
