module top;
  localparam integer K = 1;
  wire [7:0] o;
  if (1) begin : g
    if (K == 2) begin : x assign o = 8'd200; end
    else begin : x assign o = 8'd9; end
    localparam integer K = 2;
  end
  initial #1 $display("@o=%0d", o);
  initial #5 $finish;
endmodule
