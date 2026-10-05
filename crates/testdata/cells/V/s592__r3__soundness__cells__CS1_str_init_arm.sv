module top;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g string s = "a"; end
      default: begin : g string s = "b"; end
    endcase
    localparam K = 8;
  end
  initial #1 $display("@s=%s", gb.g.s);
  initial #5 $finish;
endmodule
