module top;
  function automatic logic [3:0] fx(input integer a); if (a > 5) fx = 4'd1; endfunction
  if (1) begin : gb
    if (K == 4'd0) begin : g initial #1 $display("@then K=%b", K); end
    else begin : g initial #1 $display("@else K=%b", K); end
    localparam logic [3:0] K = fx(2);
  end
  initial #5 $finish;
endmodule
