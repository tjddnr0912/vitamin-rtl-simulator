module m #(parameter P = 0) ();
  case (-1)
    32'hFFFFFFFF: begin : a initial #1 $display("@%m a"); end
    P: begin : k initial #1 $display("@%m k"); end
    default: begin : d initial #1 $display("@%m def"); end
  endcase
endmodule
module top;
  m #(.P("ab")) uA ();
  m #(.P(0)) uB ();
  initial #5 $finish;
endmodule
