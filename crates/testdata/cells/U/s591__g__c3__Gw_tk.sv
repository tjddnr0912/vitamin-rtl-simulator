module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    int r;
    task automatic t(); r = i + 0; endtask
    initial begin #1 t(); $display("tk %m r=%0d", r); end
  end
  initial #100 $finish;
endmodule
