import type { Receipt, PaperlessInvoiceRow, InvoiceRef, InvoiceData } from './types';

export interface Invoice {
	getDateTime(): string | null;
	getLiters(): number | null;
	getPrice(): number | null;
	getDisplayName(): string;
	getRef(): InvoiceRef;
	/** Inline payload sent to backend with InvoiceRef. Receipts return null (backend loads from DB). */
	getData(): InvoiceData | null;
	/** For UI: pre-selected assignment type (Fuel if invoice is fuel, else Other). */
	looksLikeFuel(): boolean;
	/** Source-specific extras the modal still needs (e.g. mismatch tooltips). */
	getRaw(): Receipt | PaperlessInvoiceRow;
}

class ReceiptInvoice implements Invoice {
	constructor(private r: Receipt) {}
	getDateTime() { return this.r.receiptDatetime; }
	getLiters()   { return this.r.liters; }
	getPrice()    { return this.r.totalPriceEur; }
	getDisplayName() { return this.r.fileName; }
	getRef(): InvoiceRef { return { source: 'receipt', id: this.r.id }; }
	getData(): InvoiceData | null { return null; }
	looksLikeFuel() { return this.r.liters !== null && this.r.liters > 0; }
	getRaw() { return this.r; }
}

class PaperlessInvoice implements Invoice {
	constructor(private p: PaperlessInvoiceRow) {}
	getDateTime() { return this.p.receiptDatetime; }
	getLiters()   { return this.p.liters; }
	getPrice()    { return this.p.totalPriceEur; }
	getDisplayName() { return this.p.title; }
	getRef(): InvoiceRef { return { source: 'paperless', id: this.p.paperlessDocumentId }; }
	getData(): InvoiceData {
		return {
			datetime: this.p.receiptDatetime,
			liters: this.p.liters,
			totalPriceEur: this.p.totalPriceEur,
			title: this.p.title,
			assignmentType: this.p.assignmentType,
		};
	}
	looksLikeFuel() { return this.p.assignmentType === 'Fuel'; }
	getRaw() { return this.p; }
}

/** The ONE type guard. Source-checking elsewhere is a smell. */
export function adaptInvoice(source: Receipt | PaperlessInvoiceRow): Invoice {
	return 'paperlessDocumentId' in source
		? new PaperlessInvoice(source)
		: new ReceiptInvoice(source);
}
