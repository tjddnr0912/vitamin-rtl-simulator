// Benchmark testbench for darkriscv (upstream BSD-3 RTL, unmodified).
// Boots the bundled src/darksocv.mem firmware image on the full darksocv SoC
// and folds core bus traffic into one deterministic DIGEST.
`timescale 1ns / 1ps

module tb;

    reg CLK = 0;
    reg RES = 1;

    integer N;
    integer cyc;
    integer pa;
    reg [63:0] digest;

    wire        TX;
    wire        RX = 1'b1;
    wire [31:0] LED;
    wire [31:0] OPORT;
    wire [3:0]  DEBUG;

    darksocv soc0
    (
        .XCLK(CLK),
        .XRES(RES),
        .UART_RXD(RX),
        .UART_TXD(TX),
        .LED(LED),
        .IPORT(32'd0),
        .OPORT(OPORT),
        .DEBUG(DEBUG)
    );

    always #5 CLK = ~CLK;   // 100 MHz

    // X-sanitiser: keeps the digest deterministic and 2-state while still
    // being sensitive to *where* X appears (a differing X map differs here).
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
        pa = $value$plusargs("N=%d", N);   // N unchanged if absent
        #1000 RES = 0;
    end

    // sample on negedge: no race with the DUT's posedge NBA updates
    always@(negedge CLK)
    begin
        if(RES==0)
        begin
            cyc = cyc + 1;

            digest = {digest[62:0], digest[63]}
                   ^ { san(soc0.bridge0.core0.IADDR),
                       san(soc0.bridge0.core0.DATAO) }
                   ^ { san(soc0.bridge0.core0.DADDR),
                       san(soc0.bridge0.core0.DATAI) }
                   ^ { 56'd0,
                       soc0.bridge0.core0.IDREQ === 1'b1,
                       soc0.bridge0.core0.DDREQ === 1'b1,
                       soc0.bridge0.core0.DRD   === 1'b1,
                       soc0.bridge0.core0.DWR   === 1'b1,
                       san({28'd0, soc0.bridge0.core0.DBE}) };

            if(cyc >= N)
            begin
                fold_regs;
                $display("\nDIGEST=%016h", digest);
                $finish;
            end
        end
    end

    // final fold of the whole architectural register file
    integer k;
    task fold_regs;
    begin
        for(k=0;k!=32;k=k+1)
        begin
            digest = {digest[62:0], digest[63]}
                   ^ {32'd0, san(soc0.bridge0.core0.REGS[k])};
        end
    end
    endtask

    // watchdog
    initial
    begin
        #40000000;
        $display("WATCHDOG");
        $display("\nDIGEST=%016h", digest);
        $finish;
    end

endmodule
