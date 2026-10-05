module top;
  function [64:0] g(); g = 65'd1; endfunction
  generate
    case (1)
      g(): begin : g_a initial $display("L25 a"); end
      default: begin : g_def initial $display("L25 def"); end
    endcase
  endgenerate
endmodule
