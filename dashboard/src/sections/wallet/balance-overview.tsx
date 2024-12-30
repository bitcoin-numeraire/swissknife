import type { Contact } from 'src/lib/swissknife';
import type { CardProps } from '@mui/material/Card';
import type { IFiatPrices } from 'src/types/bitcoin';

import { mutate } from 'swr';
import { useTabs, useBoolean } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Tab from '@mui/material/Tab';
import Card from '@mui/material/Card';
import Button from '@mui/material/Button';
import Tooltip from '@mui/material/Tooltip';
import { useTheme } from '@mui/material/styles';

import { fSats, fPercent } from 'src/utils/format-number';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { Chart, useChart } from 'src/components/chart';
import { CustomTabs } from 'src/components/custom-tabs';
import { NewInvoiceDialog, NewPaymentDialog } from 'src/components/transactions';

// ----------------------------------------------------------------------

type Props = CardProps & {
  income: TabProps;
  expenses: TabProps;
  title: string;
  tooltipTitle: string;
  totalBalance?: number;
  fiatPrices: IFiatPrices;
  isAdmin?: boolean;
  contacts?: Contact[];
};

type TabProps = {
  value: 'income' | 'expenses';
  label: string;
  tooltipTitle: string;
  percent: number;
  total: number;
  color: string;
  series: ApexAxisChartSeries;
};

export function BalanceOverview({
  sx,
  income,
  expenses,
  title,
  tooltipTitle,
  fiatPrices,
  totalBalance,
  contacts,
  isAdmin,
  ...other
}: Props) {
  const { t } = useTranslate();
  const theme = useTheme();
  const tabs = useTabs('income');
  const newInvoice = useBoolean();
  const newPayment = useBoolean();

  const chartColors =
    tabs.value === 'income' ? [theme.palette.success.main] : [theme.palette.error.main];

  const chartOptions = useChart({
    colors: chartColors,
    xaxis: {
      type: 'datetime',
      labels: {
        style: { colors: theme.palette.grey[400] },
        trim: true,
        hideOverlappingLabels: true,
        datetimeUTC: false,
      },
    },
    yaxis: {
      labels: {
        style: { colors: theme.palette.grey[400] },
        formatter: (value: number) => fSats(value),
      },
    },
    stroke: { width: 3 },
    tooltip: {
      y: { formatter: (value: number) => `${fSats(value)} sats`, title: { formatter: () => '' } },
    },
  });

  const handleOnSuccessAction = () => {
    if (isAdmin) {
      mutate(endpointKeys.invoices.list);
      mutate(endpointKeys.payments.list);
    } else {
      mutate(endpointKeys.userWallet.get);
    }
  };

  const renderBalance = (
    <Box sx={{ flexGrow: 1 }}>
      <Box
        sx={{
          mb: 1,
          gap: 0.5,
          display: 'flex',
          alignItems: 'center',
          color: 'text.secondary',
          typography: 'subtitle2',
        }}
      >
        {title}
        <Tooltip title={tooltipTitle}>
          <Iconify width={16} icon="eva:info-outline" sx={{ color: 'text.disabled' }} />
        </Tooltip>
      </Box>
      {totalBalance !== undefined && (
        <Box sx={{ typography: 'h3' }}>
          <SatsWithIcon amountMSats={totalBalance} />
        </Box>
      )}
    </Box>
  );

  const renderActions = (
    <Box sx={{ gap: 1, display: 'flex' }}>
      <Button
        onClick={newPayment.onTrue}
        variant="soft"
        size="small"
        startIcon={<Iconify width={16} icon="eva:arrow-upward-fill" />}
      >
        {t('send')}
      </Button>
      <Button
        onClick={newInvoice.onTrue}
        variant="soft"
        size="small"
        startIcon={<Iconify width={16} icon="eva:arrow-downward-fill" />}
      >
        {t('request')}
      </Button>
    </Box>
  );

  const renderTabs = (
    <CustomTabs
      value={tabs.value}
      onChange={tabs.onChange}
      variant="fullWidth"
      sx={{ my: 3, borderRadius: 2 }}
      slotProps={{
        indicator: { borderRadius: 1.5, boxShadow: theme.customShadows.z4 },
        tab: { p: 3 },
      }}
    >
      {[income, expenses].map((tab) => (
        <Tab
          key={tab.value}
          value={tab.value}
          label={
            <Box
              sx={{
                width: 1,
                display: 'flex',
                gap: { xs: 1, md: 2.5 },
                flexDirection: { xs: 'column', md: 'row' },
                alignItems: { xs: 'center', md: 'flex-start' },
              }}
            >
              <Box
                sx={{
                  width: 48,
                  height: 48,
                  flexShrink: 0,
                  borderRadius: '50%',
                  alignItems: 'center',
                  color: `${tab.color}.lighter`,
                  justifyContent: 'center',
                  bgcolor: `${tab.color}.main`,
                  display: { xs: 'none', md: 'inline-flex' },
                }}
              >
                <Iconify
                  width={24}
                  icon={
                    tab.value === 'expenses'
                      ? 'eva:diagonal-arrow-right-up-fill'
                      : 'eva:diagonal-arrow-left-down-fill'
                  }
                />
              </Box>

              <div>
                <Box
                  sx={{
                    mb: 1,
                    gap: 0.5,
                    display: 'flex',
                    alignItems: 'center',
                    typography: 'subtitle2',
                  }}
                >
                  {tab.label}
                  <Tooltip title={tab.tooltipTitle} placement="top">
                    <Iconify width={16} icon="eva:info-outline" sx={{ color: 'text.disabled' }} />
                  </Tooltip>
                </Box>

                <Box sx={{ typography: 'h4' }}>
                  <SatsWithIcon amountMSats={tab.total} />
                </Box>
              </div>

              <Label
                color={tab.percent < 0 ? 'error' : 'success'}
                startIcon={
                  <Iconify
                    width={24}
                    icon={
                      tab.percent < 0
                        ? 'solar:double-alt-arrow-down-bold-duotone'
                        : 'solar:double-alt-arrow-up-bold-duotone'
                    }
                  />
                }
                sx={{ top: 8, right: 8, position: { md: 'absolute' } }}
              >
                {tab.percent >= 0 && '+'}
                {fPercent(tab.percent)}
              </Label>
            </Box>
          }
        />
      ))}
    </CustomTabs>
  );

  return (
    <Card sx={{ p: 3, ...sx }} {...other}>
      <Box
        sx={{
          gap: 2,
          display: 'flex',
          alignItems: 'flex-start',
          flexDirection: { xs: 'column', md: 'row' },
        }}
      >
        {renderBalance}
        {renderActions}
      </Box>

      {renderTabs}

      <Chart
        type="line"
        series={tabs.value === 'income' ? income.series : expenses.series}
        options={chartOptions}
        sx={{ height: 270 }}
      />

      <NewInvoiceDialog
        fiatPrices={fiatPrices}
        open={newInvoice.value}
        onClose={newInvoice.onFalse}
        onSuccess={handleOnSuccessAction}
        isAdmin={isAdmin}
      />

      <NewPaymentDialog
        contacts={contacts}
        fiatPrices={fiatPrices}
        open={newPayment.value}
        onClose={newPayment.onFalse}
        onSuccess={handleOnSuccessAction}
        isAdmin={isAdmin}
      />
    </Card>
  );
}
