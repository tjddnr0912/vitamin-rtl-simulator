module top;
  function automatic integer fy(input integer a); integer r; if (a > 5) r = 1; fy = r + 4; endfunction
  if (1) begin : gb
    wire [K-1:0] w = '1;
    initial #1 $display("@bits=%0d K=%0d", $bits(w), K);
    localparam integer K = fy(2);
  end
  initial #5 $finish;
endmodule
