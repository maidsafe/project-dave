export interface SmartAccountInfo {
    address: string;
    antBalance: bigint;
    exists: boolean;
}

export interface PaymasterCostEstimateOptions {
    currentSmartAccountBalance?: bigint;
}

export interface PaymasterCostEstimate {
    // Payment costs
    totalPaymentAmount: bigint;
    paymentBatchCount: number;

    // Gas costs (in ANT)
    estimatedPaymentGasCost: bigint;
    estimatedFundingGasCost: bigint;
    totalGasCost: bigint;

    // Required amounts
    requiredInSmartAccount: bigint;
    currentSmartAccountBalance: bigint;
    fundingRequired: boolean;
    fundingAmount: bigint; // Amount that needs to be transferred from EOA
}

export type PaymasterFlowStep =
    | 'checking-smart-account'
    | 'smart-account-setup'
    | 'cost-estimation'
    | 'funding-smart-account'
    | 'executing-payments'
    | 'completed'
    | 'error';

export interface PaymasterFlowState {
    currentStep: PaymasterFlowStep;
    smartAccount: SmartAccountInfo | null;
    costEstimate: PaymasterCostEstimate | null;
    userFundingAmount: bigint | null; // User-selected funding amount
    error: string | null;
    isProcessing: boolean;
}
