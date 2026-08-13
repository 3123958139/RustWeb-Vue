import {defineStore} from 'pinia'
import {computed, reactive, ref} from 'vue'
import type {DynoFields, EcuFields, FluxFields} from '@shared/api/generated'

interface EnvParameter {
    label: string
    value: number
    unit: string
}

interface ControlPanelState {
    machNumber: number
    altitude: number
    throttleDuty: number
    wheelLoad: string
}

interface FooterStats {
    ecuRxBytes: number
    ecuRxFrames: number
    adam4015RxBytes: number
    adam4117RxBytes: number
    dynoRxBytes: number
    fluxRxBytes: number
    lastSentHex: string
    lastSentName: string
}

export const useDashboardStore = defineStore('fj200c_main-dashboard', () => {
    const ecuData = reactive<EcuFields>({
        ngSpeed: 0,
        exhaustTemp: 0,
        npSpeed: 0,
        machNumber: 0,
        altitude: 0,
        intakeTemp: 0,
        throttle: 0,
        engineStatus: '',
        workingVoltage: 0,
        cmdExecStatus: '',
        faultCode1: 0,
        faultCode2: 0,
        oilPressure: 0,
        oilTemp: 0,
        fuelPressure: 0,
        accessoryStatus: '',
        exchangerOutletTemp: 0,
        fingerprintCode: '',
        frameCount: 0,
        stopSolenoid: false,
        fuelPump: false,
        oilPump: false,
        ignition: false,
        starter: false,
        wheelLoadStatus: false,
        faultCodes: {
            fc1SelfCheckExhaust: false,
            fc1SelfCheckIntakeTemp: false,
            fc1SelfCheckOilPressure: false,
            fc1SelfCheckOilTemp: false,
            fc1SelfCheckFuelPressure: false,
            fc1SelfCheckNgSpeed: false,
            fc1SelfCheckNpSpeed: false,
            fc1SelfCheckFuelVent: false,
            fc1ColdStartAbnormal: false,
            fc1IgnitionFailure: false,
            fc1Overtemp: false,
            fc1StartTimeout: false,
            fc1StartSpeedLow: false,
            fc1NgOverspeed: false,
            fc1NpOverspeed: false,
            fc1ExhaustOvertemp: false,
            fc2NgSpeedFault: false,
            fc2NpSpeedFault: false,
            fc2ExhaustTempFault: false,
            fc2OilTempFault: false,
            fc2OilPressureFault: false,
            fc2FuelPressureFault: false,
            fc2VoltageAbnormal: false,
            fc2StartVoltageAbnormal: false,
            fc2GenVoltageAbnormal: false,
            fc2InFlightFlameout: false,
            fc2CommDisconnected: false,
        },
        engineStatusU8: '',
        cmdExecU8: '',
    })

    const envParams = reactive<EnvParameter[]>([
        {label: '大气温度', value: 0, unit: '℃'},
        {label: '大气湿度', value: 0, unit: '%'},
        {label: '大气压力', value: 0, unit: 'KPa'},
        {label: '进口温度', value: 0, unit: '℃'},
        {label: '燃油流量', value: 0, unit: 'L/h'},
        {label: '扭矩转速', value: 0, unit: 'r/min'},
        {label: '扭矩', value: 0, unit: 'N·m'},
        {label: '扭矩功率', value: 0, unit: 'kW'},
    ])

    const dynoData = reactive<DynoFields>({
        jkwd: 0,
        njzs: 0,
        nj: 0,
        njgl: 0,
    })

    const fluxData = reactive<FluxFields>({
        ll: 0,
    })

    const dashboardState = computed(() => ({
        ngSpeed: ecuData.ngSpeed ?? 0,
        exhaustTemp: ecuData.exhaustTemp ?? 0,
        npSpeed: ecuData.npSpeed ?? 0,
        fuelFlow: fluxData.ll ?? 0,
        dynamometerPower: dynoData.njgl ?? 0,
    }))

    const controlPanel = reactive<ControlPanelState>({
        machNumber: 0,
        altitude: 0,
        throttleDuty: 0,
        wheelLoad: '0',
    })

    const footerStats = reactive<FooterStats>({
        ecuRxBytes: 0,
        ecuRxFrames: 0,
        adam4015RxBytes: 0,
        adam4117RxBytes: 0,
        dynoRxBytes: 0,
        fluxRxBytes: 0,
        lastSentHex: '',
        lastSentName: '',
    })

    const isSimulating = ref(false)
    const isRecording = ref(false)
    const isDark = ref(true)

    const chartData = ref<Array<{
        ngSpeed: number
        exhaustTemp: number
        dynamometerPower: number
        fuelFlow: number
        npSpeed: number
    }>>([])
    const chartTime = ref<string[]>([])

    function addChartPoint() {
        const ds = dashboardState.value
        chartData.value.push({
            ngSpeed: ds.ngSpeed,
            exhaustTemp: ds.exhaustTemp,
            dynamometerPower: ds.dynamometerPower,
            fuelFlow: ds.fuelFlow,
            npSpeed: ds.npSpeed,
        })
        const d = new Date()
        chartTime.value.push(d.toLocaleTimeString())
        if (chartData.value.length > 100) {
            chartData.value.shift()
            chartTime.value.shift()
        }
    }

    return {
        ecuData,
        envParams,
        dynoData,
        fluxData,
        dashboardState,
        faultCodes: computed(() => ecuData.faultCodes),
        controlPanel,
        footerStats,
        chartData,
        chartTime,
        addChartPoint,
        isSimulating,
        isRecording,
        isDark,
    }
})
