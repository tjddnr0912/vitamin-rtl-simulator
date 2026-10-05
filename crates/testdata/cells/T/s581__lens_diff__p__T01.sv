module m #(parameter int W = 4, parameter logic [W-1:0] P = '1) ();
  case (P)
    8'hFF: begin : a initial $display("@%m a"); end
    4'hF:  begin : b initial $display("@%m b"); end
    default: begin : d initial $display("@%m def"); end
  endcase
endmodule
module top;
  m #(.W(8)) u8 ();
  m u4 ();
  m #(.W(8), .P(8'h0F)) u8f ();
  m #(.W(12)) u12 ();
endmodule
