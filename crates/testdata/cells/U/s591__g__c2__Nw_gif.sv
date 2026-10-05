module top;
  localparam P = 3;
  if (1) begin : b
    localparam [64:0] P = 65'h1_0000_0000_0000_0009;
    if (P > 65'd100) begin : t initial #1 $display("gif big"); end
    else begin : e initial #1 $display("gif small"); end
  end
  initial #100 $finish;
endmodule
