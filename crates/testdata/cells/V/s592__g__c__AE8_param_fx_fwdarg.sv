module top;
  function automatic logic [3:0] fx(input integer a); if (a > 5) fx = 4'd1; endfunction
  if (1) begin : gb
    localparam logic [3:0] P = fx(A);
    case (P)
      4'd0: begin : g initial #1 $display("@zero P=%b", P); end
      default: begin : g initial #1 $display("@def P=%b", P); end
    endcase
    localparam A = 2;
  end
  initial #5 $finish;
endmodule
