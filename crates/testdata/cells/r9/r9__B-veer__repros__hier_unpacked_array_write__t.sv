module ram;
  reg [7:0] ram_core [0:3];
endmodule
module top;
  ram u ();
  initial begin
    u.ram_core = '{default: 8'h11};    // hierarchical whole-array write with a default pattern
    u.ram_core[2] = 8'h22;             // hierarchical element write
    #1 $display("%h %h %h", u.ram_core[0], u.ram_core[2], u.ram_core[3]);
    $finish;
  end
endmodule
