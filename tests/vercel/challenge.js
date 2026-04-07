"use strict";
var o = new TextEncoder("utf-8");
var v = new TextDecoder("utf-8");
var g = new DataView(new ArrayBuffer(8));
var u = [];
var m = "runtime.ticks";
var h = "runtime.sleepTicks";
var U = "syscall/js.finalizeRef";
var l = "syscall/js.stringVal";
var f = "syscall/js.valueGet";
var r = "syscall/js.valueSet";
var e = "syscall/js.valueDelete";
var n = "syscall/js.valueIndex";
var t = "syscall/js.valueSetIndex";
var c = "syscall/js.valueCall";
var w = "syscall/js.valueInvoke";
var P = "syscall/js.valueNew";
var E0 = "syscall/js.valueLength";
var E1 = "syscall/js.valuePrepareString";
var E2 = "syscall/js.valueLoadString";
var E3 = "syscall/js.valueInstanceOf";
var E4 = "syscall/js.copyBytesToGo";
var E5 = "syscall/js.copyBytesToJS";
var E6 = class {
    constructor() {
        const Ek = {
            jzGjt: function (EK, Ev) {
                return EK === Ev;
            },
            bDkxX: function (EK, Ev) {
                return EK(Ev);
            },
            nzznO: "number",
            wRULU: function (EK, Ev) {
                return EK << Ev;
            },
            BTrUL: function (EK, Ev) {
                return EK | Ev;
            },
            eFcjW: function (EK, Ev) {
                return EK << Ev;
            },
            DvkMN: function (EK, Ev) {
                return EK << Ev;
            },
            jhJdA: function (EK, Ev) {
                return EK === Ev;
            },
            UXOFp: function (EK, Ev) {
                return EK(Ev);
            },
            gmPAh: "string",
            HSSOG: "function",
            rpfBj: function (EK) {
                return EK();
            },
            JtCAt: function (EK, Ev) {
                return EK < Ev;
            },
            RMDGa: function (EK, Ev) {
                return EK + Ev;
            },
            OqAUp: function (EK, Ev) {
                return EK == Ev;
            },
            bGwdr: function (EK, Ev) {
                return EK < Ev;
            },
            bTHqV: function (EK, Ev) {
                return EK + Ev;
            },
            REymQ: function (EK, Ev) {
                return EK * Ev;
            },
            RDUbi: function (EK) {
                return EK();
            },
            lvYri: function (EK, Ev) {
                return EK + Ev;
            },
            RNYcx: function (EK, Ev) {
                return EK < Ev;
            },
            xcndd: function (EK, Ev) {
                return EK != Ev;
            },
            eTNVm: "invalid file descriptor:",
            sleQy: function (EK, Ev) {
                return EK + Ev;
            },
            doTVm: function (EK, Ev, ET) {
                return EK(Ev, ET);
            },
            HzexX: "syscall/js.finalizeRef not implemented",
            DUVoo: function (EK, Ev) {
                return EK(Ev);
            },
            PLvXc: function (EK, Ev) {
                return EK(Ev);
            },
            Tkliw: function (EK, Ev) {
                return EK(Ev);
            },
            KxBjO: function (EK, Ev, ET) {
                return EK(Ev, ET);
            },
            vAZIh: function (EK, Ev) {
                return EK(Ev);
            },
            TdmrP: function (EK, Ev) {
                return EK(Ev);
            },
            Pinxd: function (EK, Ev, ET, ES) {
                return EK(Ev, ET, ES);
            },
            eTzSP: function (EK, Ev) {
                return EK + Ev;
            },
            JOSZv: function (EK, Ev, ET) {
                return EK(Ev, ET);
            },
            hQmAG: function (EK) {
                return EK();
            },
            GRLmS: function (EK, Ev, ET, ES) {
                return EK(Ev, ET, ES);
            },
            NPcHr: function (EK) {
                return EK();
            },
            huNQu: function (EK, Ev, ET) {
                return EK(Ev, ET);
            },
            xOEev: function (EK, Ev) {
                return EK(Ev);
            },
            RkzDV: function (EK) {
                return EK();
            },
            YJfMp: function (EK, Ev, ET, ES) {
                return EK(Ev, ET, ES);
            },
            aVHEj: function (EK, Ev, ET) {
                return EK(Ev, ET);
            },
            NtvWO: function (EK, Ev) {
                return EK instanceof Ev;
            },
            xHtca: function (EK) {
                return EK();
            },
            JZFIE: function (EK) {
                return EK();
            },
            PabOv: function (EK, Ev) {
                return EK - Ev;
            }
        };
        this._callbackTimeouts = new Map();
        this._nextCallbackTimeoutID = 1;
        let EV = () => new DataView(this._inst.exports.memory.buffer);
        let EO = EK => {
            g.setBigInt64(0, EK, !0);
            let Ev = g.getFloat64(0, !0);
            if (Ek.jzGjt(Ev, 0)) {
                return;
            }
            if (!Ek.bDkxX(isNaN, Ev)) {
                return Ev;
            }
            let ET = EK & 0xffffffffn;
            return this._values[ET];
        };
        let EZ = EK => {
            let Ev = EV().getBigUint64(EK, !0);
            return Ek.bDkxX(EO, Ev);
        };
        let Eq = EK => {
            let Ev = 0x7ff80000n;
            if (typeof EK == Ek.nzznO) {
                if (Ek.bDkxX(isNaN, EK)) {
                    return Ek.wRULU(Ev, 0x20n);
                } else if (Ek.jzGjt(EK, 0)) {
                    return Ek.BTrUL(Ek.wRULU(Ev, 0x20n), 0x1n);
                } else {
                    g.setFloat64(0, EK, true);
                    return g.getBigInt64(0, true);
                }
            }
            switch (EK) {
                case undefined:
                    return 0x0n;
                case null:
                    return Ek.BTrUL(Ek.eFcjW(Ev, 0x20n), 0x2n);
                case !0:
                    return Ek.BTrUL(Ek.eFcjW(Ev, 0x20n), 0x3n);
                case !1:
                    return Ek.DvkMN(Ev, 0x20n) | 0x4n;
            }
            let ET = this._ids.get(EK);
            if (Ek.jhJdA(ET, undefined)) {
                ET = this._idPool.pop();
                if (ET === undefined) {
                    ET = Ek.UXOFp(BigInt, this._values.length);
                }
                this._values[ET] = EK;
                this._goRefCounts[ET] = 0;
                this._ids.set(EK, ET);
            }
            this._goRefCounts[ET]++;
            let ES = 0x1n;
            switch (typeof EK) {
                case Ek.gmPAh:
                    ES = 0x2n;
                    break;
                case "symbol":
                    ES = 0x3n;
                    break;
                case Ek.HSSOG:
                    ES = 0x4n;
                    break;
            }
            return ET | Ek.DvkMN(Ek.BTrUL(Ev, ES), 0x20n);
        };
        let Es = (EK, Ev) => {
            let ET = Eq(Ev);
            Ek.rpfBj(EV).setBigUint64(EK, ET, !0);
        };
        let EA = (EK, Ev, ET) => new Uint8Array(this._inst.exports.memory.buffer, EK, Ev);
        let EX = (EK, Ev, ET) => {
            let ES = new Array(Ev);
            for (let Eg = 0; Ek.JtCAt(Eg, Ev); Eg++) {
                ES[Eg] = EZ(Ek.RMDGa(EK, Eg * 8));
            }
            return ES;
        };
        let Eo = (EK, Ev) => v.decode(new DataView(this._inst.exports.memory.buffer, EK, Ev));
        let EL = Ek.PabOv(Date.now(), performance.now());
        this.importObject = {
            wasi_snapshot_preview1: {
                fd_write: function (EK, Ev, ET, ES) {
                    let Eg = 0;
                    if (Ek.OqAUp(EK, 1)) {
                        for (let Eu = 0; Ek.bGwdr(Eu, ET); Eu++) {
                            let EW = Ek.bTHqV(Ev, Ek.REymQ(Eu, 8));
                            let EH = Ek.RDUbi(EV).getUint32(EW + 0, !0);
                            let EF = EV().getUint32(Ek.lvYri(EW, 4), !0);
                            Eg += EF;
                            for (let EB = 0; Ek.RNYcx(EB, EF); EB++) {
                                let EI = Ek.rpfBj(EV).getUint8(EH + EB);
                                if (Ek.xcndd(EI, 13)) {
                                    if (Ek.OqAUp(EI, 10)) {
                                        let Em = v.decode(new Uint8Array(u));
                                        u = [];
                                        console.log(Em);
                                    } else {
                                        u.push(EI);
                                    }
                                }
                            }
                        }
                    } else {
                        console.error(Ek.eTNVm, EK);
                    }
                    Ek.rpfBj(EV).setUint32(ES, Eg, !0);
                    return 0;
                },
                fd_close: () => 0,
                fd_fdstat_get: () => 0,
                fd_seek: () => 0,
                proc_exit: EK => {
                    throw Ek.sleQy("trying to exit with code ", EK);
                },
                random_get: (EK, Ev) => {
                    crypto.getRandomValues(EA(EK, Ev));
                    return 0;
                }
            },
            gojs: {
                [m]: () => EL + performance.now(),
                [h]: EK => {
                    Ek.doTVm(setTimeout, this._inst.exports.go_scheduler, EK);
                },
                [U]: EK => {
                    console.error(Ek.HzexX);
                },
                [l]: (EK, Ev) => {
                    let ET = Ek.doTVm(Eo, EK, Ev);
                    return Ek.DUVoo(Eq, ET);
                },
                [f]: (EK, Ev, ET) => {
                    let ES = Eo(Ev, ET);
                    let Eg = Ek.DUVoo(EO, EK);
                    let Eu = Reflect.get(Eg, ES);
                    return Ek.PLvXc(Eq, Eu);
                },
                [r]: (EK, Ev, ET, ES) => {
                    let Eg = Ek.Tkliw(EO, EK);
                    let Eu = Ek.doTVm(Eo, Ev, ET);
                    let EW = EO(ES);
                    Reflect.set(Eg, Eu, EW);
                },
                [e]: (EK, Ev, ET) => {
                    let ES = Ek.PLvXc(EO, EK);
                    let Eg = Ek.KxBjO(Eo, Ev, ET);
                    Reflect.deleteProperty(ES, Eg);
                },
                [n]: (EK, Ev) => Eq(Reflect.get(EO(EK), Ev)),
                [t]: (EK, Ev, ET) => {
                    Reflect.set(Ek.DUVoo(EO, EK), Ev, Ek.vAZIh(EO, ET));
                },
                [c]: (EK, Ev, ET, ES, Eg, Eu, EW) => {
                    let EH = Ek.TdmrP(EO, Ev);
                    let EF = Ek.doTVm(Eo, ET, ES);
                    let EB = Ek.Pinxd(EX, Eg, Eu, EW);
                    try {
                        let EI = Reflect.get(EH, EF);
                        Es(EK, Reflect.apply(EI, EH, EB));
                        EV().setUint8(Ek.eTzSP(EK, 8), 1);
                    } catch (Em) {
                        Ek.JOSZv(Es, EK, Em);
                        Ek.rpfBj(EV).setUint8(Ek.bTHqV(EK, 8), 0);
                    }
                },
                [w]: (EK, Ev, ET, ES, Eg) => {
                    try {
                        let Eu = Ek.DUVoo(EO, Ev);
                        let EW = Ek.Pinxd(EX, ET, ES, Eg);
                        Ek.JOSZv(Es, EK, Reflect.apply(Eu, undefined, EW));
                        Ek.rpfBj(EV).setUint8(Ek.sleQy(EK, 8), 1);
                    } catch (EH) {
                        Es(EK, EH);
                        Ek.hQmAG(EV).setUint8(EK + 8, 0);
                    }
                },
                [P]: (EK, Ev, ET, ES, Eg) => {
                    let Eu = Ek.vAZIh(EO, Ev);
                    let EW = Ek.GRLmS(EX, ET, ES, Eg);
                    try {
                        Ek.doTVm(Es, EK, Reflect.construct(Eu, EW));
                        Ek.NPcHr(EV).setUint8(Ek.lvYri(EK, 8), 1);
                    } catch (EH) {
                        Ek.huNQu(Es, EK, EH);
                        Ek.rpfBj(EV).setUint8(Ek.sleQy(EK, 8), 0);
                    }
                },
                [E0]: EK => EO(EK).length,
                [E1]: (EK, Ev) => {
                    let ET = Ek.xOEev(String, EO(Ev));
                    let ES = o.encode(ET);
                    Es(EK, ES);
                    Ek.RkzDV(EV).setInt32(EK + 8, ES.length, !0);
                },
                [E2]: (EK, Ev, ET, ES) => {
                    let Eg = Ek.UXOFp(EO, EK);
                    Ek.YJfMp(EA, Ev, ET, ES).set(Eg);
                },
                [E3]: (EK, Ev) => EO(EK) instanceof EO(Ev),
                [E4]: (EK, Ev, ET, ES, Eg) => {
                    let Eu = EK;
                    let EW = EK + 4;
                    let EH = Ek.aVHEj(EA, Ev, ET);
                    let EF = Ek.DUVoo(EO, Eg);
                    if (!(EF instanceof Uint8Array) && !Ek.NtvWO(EF, Uint8ClampedArray)) {
                        EV().setUint8(EW, 0);
                        return;
                    }
                    let EB = EF.subarray(0, EH.length);
                    EH.set(EB);
                    Ek.RkzDV(EV).setUint32(Eu, EB.length, !0);
                    Ek.RDUbi(EV).setUint8(EW, 1);
                },
                [E5]: (EK, Ev, ET, ES, Eg) => {
                    let Eu = EK;
                    let EW = Ek.RMDGa(EK, 4);
                    let EH = EO(Ev);
                    let EF = Ek.JOSZv(EA, ET, ES);
                    if (!Ek.NtvWO(EH, Uint8Array) && !Ek.NtvWO(EH, Uint8ClampedArray)) {
                        Ek.rpfBj(EV).setUint8(EW, 0);
                        return;
                    }
                    let EB = EF.subarray(0, EH.length);
                    EH.set(EB);
                    Ek.xHtca(EV).setUint32(Eu, EB.length, !0);
                    Ek.JZFIE(EV).setUint8(EW, 1);
                }
            }
        };
        this.importObject.env = this.importObject.gojs;
    }
    async run(Ek) {
        const EV = {
            iZFHB: "bad callback: Go program has already exited",
            IarnQ: function (EO, EZ, Eq) {
                return EO(EZ, Eq);
            }
        };
        this._inst = Ek;
        this._values = [NaN, 0, null, !0, !1, self, this];
        this._goRefCounts = [];
        this._ids = new Map();
        this._idPool = [];
        this.exited = !1;
        while (true) {
            let EO = new Promise(EZ => {
                this._resolveCallbackPromise = () => {
                    if (this.exited) {
                        throw new Error(EV.iZFHB);
                    }
                    EV.IarnQ(setTimeout, EZ, 0);
                };
            });
            this._inst.exports._start();
            if (this.exited) {
                break;
            }
            await EO;
        }
    }
    _resume() {
        if (this.exited) {
            throw new Error("Go program has already exited");
        }
        this._inst.exports.resume();
        if (this.exited) {
            this._resolveExitPromise();
        }
    }
    _makeFuncWrapper(Ek) {
        let EV = this;
        return function () {
            let EO = {
                id: Ek,
                this: this,
                args: arguments
            };
            EV._pendingEvent = EO;
            EV._resume();
            return EO.result;
        };
    }
};
var E7 = {
    token: "",
    messagePort: null
};
var E8 = new Map();
function E9(Ek) {
    const EV = {
        cbBvO: function (EZ, Eq) {
            return EZ(Eq);
        },
        dOtBL: "eval-request"
    };
    let EO = Math.random().toString(36).slice(2);
    EV.cbBvO(ED, {
        type: EV.dOtBL,
        id: EO,
        argv: Ek,
        token: E7.token
    });
    return new Promise((EZ, Eq) => {
        E8.set(EO, {
            resolve: EZ,
            reject: Eq
        });
    });
}
function EE() {
    let Ek = !1;
    let EV = new Error();
    Object.defineProperty(EV, "stack", {
        get() {
            Ek = !0;
            return "";
        }
    });
    console.log(EV);
    return Ek;
}
async function Ey() {
    const Ek = {
        XNzLC: function (Eq, Es) {
            return Eq(Es);
        },
        HpuAG: "/.well-known/vercel/security/static/challenge.v2.wasm",
        ljwWU: "syscall/js.finalizeRef"
    };
    let EV = await Ek.XNzLC(fetch, Ek.HpuAG);
    let EO = new E6();
    EO.importObject.gojs[Ek.ljwWU] = () => null;
    let {
        instance: EZ
    } = await WebAssembly.instantiateStreaming(EV, EO.importObject);
    EO.run(EZ);
    return {
        instance: EZ,
        go: EO
    };
}
async function Ei(Ek, EV, EO) {
    const EZ = {
        WvhHX: function (Es, EA, EX) {
            return Es(EA, EX);
        },
        OGLWE: "/.well-known/vercel/security/request-challenge",
        dMSwj: "Cf-Mitigated",
        DRNdD: "Cf-Ray",
        KYDyG: "Challenge blocked by Cloudflare",
        gHHeG: function (Es, EA) {
            return Es === EA;
        },
        nmhvl: "Challenge blocked",
        Ogjdj: function (Es, EA) {
            return Es === EA;
        },
        MMCBb: "Challenge not forwarded",
        MBWCJ: function (Es, EA) {
            return Es >= EA;
        },
        uyvOB: function (Es, EA) {
            return Es(EA);
        }
    };
    let Eq = await EZ.WvhHX(fetch, EZ.OGLWE, {
        method: "POST",
        headers: {
            "x-vercel-challenge-token": Ek,
            "x-vercel-challenge-solution": EV,
            "x-vercel-challenge-version": EO
        }
    });
    if (!Eq.ok) {
        if (Eq.headers.get(EZ.dMSwj)) {
            let Es = Eq.headers.get(EZ.DRNdD);
            let EA = Es ? "Ray ID: " + Es : EZ.KYDyG;
            let EX = new Error(EA);
            EX.__blocked = !0;
            throw EX;
        }
        if (Eq.status === 401 || EZ.gHHeG(Eq.status, 403)) {
            let Eo = new Error(EZ.nmhvl);
            Eo.__blocked = !0;
            throw Eo;
        }
        if (EZ.Ogjdj(Eq.status, 404)) {
            let EL = new Error(EZ.MMCBb);
            EL.__blocked = !0;
            throw EL;
        }
        if (EZ.MBWCJ(Eq.status, 700)) {
            let EK = new Error(EZ.uyvOB(String, Eq.status));
            EK.__failed = !0;
            throw EK;
        }
        throw new Error(Eq.statusText);
    }
    return Eq;
}
function ED(Ek) {
    E7.messagePort?.postMessage(Ek);
}
async function Ej(Ek) {
    const EV = {
        niWqu: function (EZ, Eq) {
            return EZ(Eq);
        },
        VlewX: function (EZ, Eq, Es, EA) {
            return EZ(Eq, Es, EA);
        },
        MTIVF: "solve-response",
        ZDsvD: function (EZ, Eq) {
            return EZ != Eq;
        },
        pOTnt: function (EZ, Eq) {
            return EZ == Eq;
        },
        FZMeT: "object",
        VqDLw: function (EZ, Eq) {
            return EZ in Eq;
        },
        yBCTl: "__blocked",
        qpqgB: function (EZ, Eq) {
            return EZ != Eq;
        },
        YmmBx: function (EZ, Eq) {
            return EZ instanceof Eq;
        },
        HfKgA: function (EZ, Eq) {
            return EZ ?? Eq;
        }
    };
    await Ey();
    let EO;
    try {
        let EZ = await EV.niWqu(Solve, Ek.token);
        EO = JSON.parse(EZ);
        let Eq = EO.solution;
        await EV.VlewX(Ei, Ek.token, Eq, Ek.version);
        EV.niWqu(ED, {
            type: EV.MTIVF,
            success: !0,
            token: E7.token
        });
    } catch (Es) {
        let EA = EV.ZDsvD(Es, null) && EV.pOTnt(typeof Es, EV.FZMeT) && EV.VqDLw(EV.yBCTl, Es);
        let EX = EV.qpqgB(Es, null) && typeof Es == "object" && EV.VqDLw("__failed", Es);
        if (EA) {
            let Eo = EV.YmmBx(Es, Error) ? Es.message : EV.niWqu(String, Es);
            ED({
                type: EV.MTIVF,
                success: !1,
                blocked: !0,
                metadata: EV.HfKgA(Eo, undefined),
                token: E7.token
            });
        } else if (EO?.badInfo) {
            ED({
                type: EV.MTIVF,
                success: !1,
                blocked: !1,
                metadata: EO?.badInfo ?? undefined,
                token: E7.token
            });
        } else if (EX) {
            let EL = Es instanceof Error ? Es.message : String(Es);
            ED({
                type: EV.MTIVF,
                success: !1,
                blocked: !1,
                metadata: EV.HfKgA(EL, undefined),
                token: E7.token
            });
        } else {
            EV.niWqu(ED, {
                type: "solve-response",
                success: !1,
                blocked: !1,
                metadata: undefined,
                token: E7.token
            });
        }
    }
}
function Ed(Ek) {
    const EV = {
        UiHRr: function (EO, EZ) {
            return EO(EZ);
        },
        dDnbE: "solve-request",
        iSfiq: function (EO, EZ) {
            return EO in EZ;
        },
        QAQIk: "value"
    };
    if (EV.UiHRr(Ea, Ek)) {
        switch (Ek.data.type) {
            case EV.dDnbE:
                {
                    EV.UiHRr(Ej, Ek.data);
                    break;
                }
            case "eval-response":
                {
                    let EO = E8.get(Ek.data.id);
                    if (EV.iSfiq(EV.QAQIk, Ek.data)) {
                        EO?.resolve(Ek.data.value);
                    } else {
                        EO?.reject(Ek.data.error);
                    }
                    E8.delete(Ek.data.id);
                    break;
                }
            default:
                break;
        }
    }
}
function Ea(Ek) {
    const EV = {
        PfGbX: function (EO, EZ) {
            return EO === EZ;
        }
    };
    E7.token = E7.token || Ek.data.token;
    return EV.PfGbX(Ek.data.token, E7.token);
}
self.onmessage = Ek => {
    let EV = Ek.data.port;
    E7.messagePort = EV;
    EV.onmessage = Ed;
};
self.setTimeout.e = E9;
self.setTimeout.d = EE;