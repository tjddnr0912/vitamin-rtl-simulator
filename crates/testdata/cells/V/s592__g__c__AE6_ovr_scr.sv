module m #(parameter logic [3:0] P = 4'd7) ();
  if (1) begin : gb
    case (K)
      4'd0: begin : g initial #1 $display("@zero K=%b", K); end
      default: begin : g initial #1 $display("@def K=%b", K); end
    endcase
    localparam logic [3:0] K = P;
  end
endmodule
module top;
  function automatic logic [3:0] fx(input integer a); if (a > 5) fx = 4'd1; endfunction
  m #(.P(fx(2))) u ();
  initial #5 $finish;
endmodule
