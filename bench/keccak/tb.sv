`timescale 1ns/1ps
module tb;
    reg clk = 1'b0, rst_n = 1'b0, start = 1'b0;
    reg  [1599:0] din;
    wire [1599:0] dout;
    wire done;
    integer i, n;
    integer nperm;
    integer got;
    reg [63:0] acc;

    keccak_f u (.clk(clk), .rst_n(rst_n), .start(start), .din(din), .dout(dout), .done(done));

    always #5 clk = ~clk;

    initial begin
        got = $value$plusargs("N=%d", nperm);
        if (got == 0) nperm = 100;
        din = 1600'd0;
        @(posedge clk); @(posedge clk);
        rst_n = 1'b1;
        @(posedge clk);
        for (n = 0; n < nperm; n = n + 1) begin
            start = 1'b1;
            @(posedge clk);
            start = 1'b0;
            wait (done == 1'b1);
            @(posedge clk);
            // chain: next input = previous output, lane 0 stirred by the counter
            din = dout;
            din[63:0] = din[63:0] ^ n;
        end
        acc = 64'd0;
        for (i = 0; i < 25; i = i + 1) acc = acc ^ dout[64*i +: 64];
        $display("perms=%0d lane0=%h lane1=%h acc=%h", nperm, dout[63:0], dout[127:64], acc);
        $finish;
    end
endmodule
