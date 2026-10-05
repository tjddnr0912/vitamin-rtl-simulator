module m #(parameter type T = logic [3:0]) ();
  localparam T TP = '1;
  localparam T TQ = -1;
  initial $display("@%m TP=%0d bits=%0d neg=%0d TQ=%0d negQ=%0d", TP, $bits(TP), TP < 0, TQ, TQ < 0);
  if (TP < 0) begin : gi initial $display("@%m gen-if neg"); end else begin : ge initial $display("@%m gen-if nonneg"); end
  initial case (-1) TP: $display("@%m proc-case hit"); default: $display("@%m proc-case def"); endcase
  case (-1) TQ: begin : a initial $display("@%m gc-TQ hit"); end default: begin : ad initial $display("@%m gc-TQ def"); end endcase
endmodule
module top;
  m #(.T(logic signed [3:0])) u_s4 ();
  m #(.T(byte)) u_byte ();
endmodule
