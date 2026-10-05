module top;
  if (1) begin : gb
    typedef enum logic [K-1:0] {EA, EB} e_t;
    e_t v = EB;
    initial #1 $display("@bits=%0d v=%0d", $bits(v), v);
    localparam K = 6;
  end
  initial #5 $finish;
endmodule
