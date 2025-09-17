/**
 * Format a balance string to a more readable format
 * @param balance - The balance as a string
 * @param decimals - Number of decimal places to show (default: 4)
 * @returns Formatted balance string
 */
export function formatBalance(balance: string, decimals: number = 4): string {
    const num = parseFloat(balance);
    
    if (isNaN(num)) {
        return '0';
    }
    
    // For very small numbers, show more decimals
    if (num > 0 && num < 0.0001) {
        return num.toFixed(8);
    }
    
    // For numbers less than 1, show up to 6 decimals
    if (num < 1) {
        return num.toFixed(6).replace(/\.?0+$/, '');
    }
    
    // For larger numbers, use standard formatting
    return num.toFixed(decimals).replace(/\.?0+$/, '');
}

/**
 * Format ANT token balance with appropriate precision
 * @param balance - The ANT balance as a string
 * @returns Formatted ANT balance with "ANT" suffix
 */
export function formatAntBalance(balance: string): string {
    const formatted = formatBalance(balance);
    return `${formatted} ANT`;
}

/**
 * Format ETH balance with appropriate precision
 * @param balance - The ETH balance as a string
 * @returns Formatted ETH balance with "ETH" suffix
 */
export function formatEthBalance(balance: string): string {
    const formatted = formatBalance(balance, 6);
    return `${formatted} ETH`;
}

/**
 * Format a balance for display with loading state
 * @param balance - The balance as a string
 * @param isLoading - Whether the balance is currently loading
 * @param formatter - Optional custom formatter function
 * @returns Formatted balance or loading indicator
 */
export function formatBalanceWithLoading(
    balance: string, 
    isLoading: boolean, 
    formatter?: (balance: string) => string
): string {
    if (isLoading) {
        return '...';
    }
    
    if (formatter) {
        return formatter(balance);
    }
    
    return formatBalance(balance);
}