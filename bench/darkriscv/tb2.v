// Core-level harness: upstream darkriscv + darkram only (no UART/IO/PLL).
// Same bundled src/darksocv.mem image; wiring copied verbatim from
// darkbridge.v's __HARVARD__ port map (darklife/darkriscv, BSD-3-Clause,
// Copyright (c) 2018 Marcelo Samsoniuk).  Upstream RTL itself is unmodified.
`timescale 1ns / 1ps

module tb2;

    reg CLK = 0;
    reg RES = 1;

    integer N, cyc, pa, k;
    reg [63:0] digest;

    wire        IDREQ, IDACK;
    wire [31:0] IADDR, IDATA;
    wire        DDREQ, DDACK, DRW, DRD, DWR;
    wire [31:0] DADDR, DATAO, DATAI;
    wire [3:0]  DBE;
    wire [3:0]  KDEBUG, RDEBUG;

    darkriscv #(.CPTR(0)) core0
    (
        .CLK(CLK), .RES(RES),
        .IDREQ(IDREQ), .IDATA(IDATA), .IADDR(IADDR), .IDACK(IDACK), .IBERR(1'b0),
        .DADDR(DADDR), .DATAI(DATAI), .DATAO(DATAO), .DBE(DBE),
        .DRW(DRW), .DWR(DWR), .DRD(DRD), .DDREQ(DDREQ), .DDACK(DDACK), .DBERR(1'b0),
`ifdef SIMULATION
        .ESIMREQ(1'b0),
`endif
        .DEBUG(KDEBUG)
    );

    darkram bram0
    (
        .CLK(CLK), .RES(RES), .HLT(1'b0),
        .IDREQ(IDREQ), .IADDR(IADDR), .IDATA(IDATA), .IDACK(IDACK),
        .XDREQ(DDREQ), .XRD(DRD), .XWR(DWR), .XBE(DBE),
        .XADDR(DADDR), .XATAI(DATAO), .XATAO(DATAI), .XDACK(DDACK),
        .DEBUG(RDEBUG)
    );

    always #5 CLK = ~CLK;

    function [31:0] san;
        input [31:0] v;
        begin
            san = (^v === 1'bx) ? 32'ha5a5a5a5 : v;
        end
    endfunction

    initial
    begin
        digest = 64'd0;
        cyc    = 0;
        N      = 300000;
        pa = $value$plusargs("N=%d", N);
        #1000 RES = 0;
    end

    always@(negedge CLK)
    begin
        if(RES==0)
        begin
            cyc = cyc + 1;
            digest = {digest[62:0], digest[63]}
                   ^ { san(IADDR), san(DATAO) }
                   ^ { san(DADDR), san(DATAI) }
                   ^ { 56'd0, IDREQ===1'b1, DDREQ===1'b1, DRD===1'b1, DWR===1'b1,
                       san({28'd0,DBE}) };
            if(cyc >= N)
            begin
                for(k=0;k!=32;k=k+1)
                    digest = {digest[62:0], digest[63]} ^ {32'd0, san(core0.REGS[k])};
                $display("\nDIGEST=%016h", digest);
                $finish;
            end
        end
    end

    initial
    begin
        #40000000;
        $display("WATCHDOG");
        $display("\nDIGEST=%016h", digest);
        $finish;
    end

endmodule
