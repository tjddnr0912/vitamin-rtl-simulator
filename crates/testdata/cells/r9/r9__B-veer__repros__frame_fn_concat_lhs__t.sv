module top;
  function automatic string f(input [31:0] op);
    int imm;
    imm = 0;
    {imm[5:4], imm[9:6], imm[2], imm[3]} = op[12:5];          // concatenation as the assignment target
    return $sformatf("imm=%0h", imm);
  endfunction
  string s;
  initial begin
    s = f(32'h0000_1FE0);
    $display("%s", s);
    $finish;
  end
endmodule
