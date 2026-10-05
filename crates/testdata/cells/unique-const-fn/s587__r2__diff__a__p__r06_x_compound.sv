module top;
  function automatic logic [3:0] fa(input int a);
    if (a == 1) fa = 4'd10;
    fa += 4'd1;
  endfunction
  function automatic logic [3:0] fb(input int a);
    if (a == 1) fb = 4'd10;
    fb++;
  endfunction
  localparam logic [3:0] PA = fa(2);
  localparam logic [3:0] PB = fb(2);
  initial begin #1 $display("PA=%b PB=%b", PA, PB); $finish; end
endmodule
